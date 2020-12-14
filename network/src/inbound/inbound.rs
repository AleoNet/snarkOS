// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    external::{message::Message, message_types::*, Channel, MessageName},
    inbound::Response,
    Environment,
    NetworkError,
    Receiver,
    Sender,
};

use std::{
    collections::HashMap,
    fmt::Display,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{Mutex, RwLock},
    task,
};

/// The map of remote addresses to their active read channels.
pub type Channels = HashMap<SocketAddr, Channel>;

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct Inbound {
    /// The producer for sending inbound messages to the server.
    sender: Sender,
    /// The consumer for receiving inbound messages to the server.
    receiver: Arc<Mutex<Receiver>>,
    /// The map of remote addresses to their active read channels.
    channels: Arc<RwLock<Channels>>,
    /// A counter for the number of received responses the handler processes.
    receive_response_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that succeeded.
    receive_success_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that failed.
    receive_failure_count: Arc<AtomicU64>,
}

impl Default for Inbound {
    fn default() -> Self {
        // Initialize the sender and receiver.
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);

        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            channels: Default::default(),
            receive_response_count: Default::default(),
            receive_success_count: Default::default(),
            receive_failure_count: Default::default(),
        }
    }
}

impl Inbound {
    pub async fn listen(&self, environment: &mut Environment) -> Result<(), NetworkError> {
        let (listener_address, listener) = if let Some(addr) = environment.local_address() {
            let listener = TcpListener::bind(&addr).await?;
            (addr, listener)
        } else {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let listener_address = listener.local_addr()?;
            environment.set_local_address(listener_address);
            (listener_address, listener)
        };
        info!("Listening at {}", listener_address);

        let inbound = self.clone();
        let environment = environment.clone();
        task::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((channel, remote_address)) => {
                        info!("Got a connection request from {}", remote_address);

                        let height = environment.current_block_height().await;
                        if let Some((channel, discovered_local_address)) = inbound
                            .connection_request(height, remote_address, channel)
                            .await
                            .unwrap()
                        {
                            // TODO (howardwu): Enable this peer address discovery again.
                            // // Bootstrap discovery of local node IP via VERACK responses
                            //     let local_address = peers.local_address();
                            //     if local_address != discovered_local_address {
                            //         peers.set_local_address(discovered_local_address).await;
                            //         info!("Discovered local address: {:?}", local_address);
                            //     }

                            let inbound = inbound.clone();
                            let channel = channel.clone();
                            tokio::spawn(async move {
                                inbound.inbound(listener_address, channel).await.unwrap();
                                // inbound.inbound(&discovered_local_address, channel).await?;
                            });
                        }
                    }
                    Err(error) => error!("Failed to accept connection request\n{}", error),
                }
            }
        });

        Ok(())
    }

    async fn inbound(&self, local_address: SocketAddr, channel: Channel) -> Result<(), NetworkError> {
        let mut failure_count = 0u8;
        let mut disconnect_from_peer = false;
        let mut channel = channel;
        loop {
            // Initialize the failure indicator.
            let mut failure = false;

            // warn!(
            //     "LISTENING AT {} {:?} {:?}",
            //     channel.remote_address,
            //     channel.reader.lock().await,
            //     channel.writer.lock().await
            // );

            // Read the next message from the channel. This is a blocking operation.
            let (message_name, message_bytes) = match channel.read().await {
                Ok((message_name, message_bytes)) => (message_name, message_bytes),
                Err(error) => {
                    error!("Failed to read message from channel\n{}", error);
                    Self::handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error).await;
                    // Determine if we should send a disconnect message.
                    match disconnect_from_peer {
                        true => (MessageName::from("disconnect"), vec![]),
                        false => continue,
                    }
                }
            };

            // Messages are received by a single tokio MPSC receiver with
            // the message name, bytes, and associated channel.
            //
            // The oneshot sender lets the connection thread know when the message is handled.
            let name = message_name;
            let bytes = message_bytes;

            if name == Block::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::Block(channel.remote_address, message, true)).await;
            } else if name == SyncBlock::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::Block(channel.remote_address, message, false))
                    .await;
            } else if name == GetBlock::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::GetBlock(channel.remote_address, message)).await;
            } else if name == GetMemoryPool::name() && Self::parse::<GetMemoryPool>(&bytes).is_ok() {
                self.route(Response::GetMemoryPool(channel.remote_address)).await;
            } else if name == MemoryPool::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::MemoryPool(message)).await;
            } else if name == GetSync::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::GetSync(channel.remote_address, message)).await;
            } else if name == Sync::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::Sync(channel.remote_address, message)).await;
            } else if name == Transaction::name() {
                let message = Self::parse(&bytes)?;
                self.route(Response::Transaction(channel.remote_address, message)).await;
            } else if name == GetPeers::name() && Self::parse::<GetPeers>(&bytes).is_ok() {
                self.route(Response::GetPeers(channel.remote_address)).await;
            } else if name == Peers::name() {
                let message = Self::parse::<Peers>(&bytes)?;
                self.route(Response::Peers(channel.remote_address, message)).await;
            } else if name == Version::name() {
                let message = Self::parse::<Version>(&bytes)?;
                // TODO (raychu86) Does `receive_version` need to return a channel?
                match self.receive_version(local_address, message, channel.clone()).await {
                    Ok(returned_channel) => channel = returned_channel,
                    Err(err) => error!("Failed to route response for a message\n{}", err),
                }
            } else if name == Verack::name() {
                let message = Self::parse::<Verack>(&bytes)?;
                self.route(Response::Verack(channel.remote_address, message)).await;
            } else if name == MessageName::from("disconnect") {
                info!("Disconnected from peer {:?}", channel.remote_address);
                self.route(Response::DisconnectFrom(channel.remote_address)).await;
            } else {
                debug!("Message name not recognized {:?}", name.to_string());
            }
            // TODO (howardwu): Remove this and rearchitect how disconnects are handled using the peer manager.
            // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
            // Break out of the loop if the peer disconnects.
            if disconnect_from_peer {
                warn!("Disconnecting from an unreliable peer");
                break;
            }
        }

        Ok(())
    }

    /// Logs the failure and determines whether to disconnect from a peer.
    async fn handle_failure<T: Display>(
        failure: &mut bool,
        failure_count: &mut u8,
        disconnect_from_peer: &mut bool,
        error: T,
    ) {
        // Determines the criteria for disconnecting from a peer.
        fn should_disconnect(failure_count: &u8) -> bool {
            // Tolerate up to 10 failed communications.
            *failure_count >= 10
        }

        // Only increment failure_count if we haven't seen a failure yet.
        if !*failure {
            // Update the state to reflect a new failure.
            *failure = true;
            *failure_count += 1;
            warn!(
                "Connection errored {} time(s) (error message: {})",
                failure_count, error
            );

            // Determine if we should disconnect.
            *disconnect_from_peer = should_disconnect(failure_count);
        } else {
            debug!("Connection errored again in the same loop (error message: {})", error);
        }

        // Sleep for 10 seconds
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    #[inline]
    fn parse<M: Message>(buffer: &[u8]) -> Result<M, NetworkError> {
        // TODO (howardwu): Remove usage of `to_vec`, wasteful convention and
        //  requires a function signature change to fix.
        match M::deserialize(buffer.to_vec()) {
            Ok(message) => Ok(message),
            Err(error) => {
                error!("Failed to deserialize a {}-byte message\n{}", buffer.len(), error);
                Err(NetworkError::InboundDeserializationFailed)
            }
        }
    }

    #[inline]
    async fn route(&self, response: Response) {
        if let Err(err) = self.sender.send(response).await {
            error!("Failed to route response for a message\n{}", err);
            // error!("Failed to route `{}` message from {}\n{}", name, remote_address, err);
        }
    }

    #[inline]
    pub(crate) fn receiver(&self) -> Arc<Mutex<Receiver>> {
        self.receiver.clone()
    }

    /// A connected peer has sent handshake request.
    /// Update peer's channel.
    /// If peer's block height is greater than ours, send a sync request.
    ///
    /// This method may seem redundant to handshake protocol functions but a peer can send additional
    /// Version messages if they want to update their ip address/port or want to share their chain height.
    async fn receive_version(
        &self,
        local_address: SocketAddr,
        version: Version,
        channel: Channel,
    ) -> Result<Channel, NetworkError> {
        let remote_address = SocketAddr::new(channel.remote_address.ip(), version.sender.port());

        if local_address != remote_address {
            // Route version message to peer manager.
            self.route(Response::VersionToVerack(remote_address, version.clone()))
                .await;

            // TODO (howardwu): Implement this.
            {
                // // If our peer has a longer chain, send a sync message
                // if version.height > environment.storage_read().await.get_current_block_height() {
                //     debug!("Received a version message with a greater height {}", version.height);
                //     // Update the sync node if the sync_handler is idle and there are no requested block headers
                //     if let Ok(mut sync_handler) = environment.sync_manager().await.try_lock() {
                //         if !sync_handler.is_syncing()
                //             && (sync_handler.block_headers.len() == 0 && sync_handler.pending_blocks.is_empty())
                //         {
                //             debug!("Attempting to sync with peer {}", remote_address);
                //             sync_handler.sync_node_address = remote_address;
                //
                //             if let Ok(block_locator_hashes) = environment.storage_read().await.get_block_locator_hashes() {
                //                 channel.write(&GetSync::new(block_locator_hashes)).await?;
                //             }
                //         } else {
                //             // TODO (howardwu): Implement this.
                //             {
                //                 // if let Some(channel) = environment
                //                 //     .peers_read()
                //                 //     .await
                //                 //     .get_channel(&sync_handler.sync_node_address)
                //                 // {
                //                 //     sync_handler.increment(channel.clone()).await?;
                //                 // }
                //             }
                //         }
                //     }
                // }
            }
        }
        Ok(channel)
    }

    ///
    /// Receives a connection request with a given version message.
    ///
    /// Listens for the first message request from a remote peer.
    ///
    /// If the message is a Version:
    ///
    ///     1. Create a new handshake.
    ///     2. Send a handshake response.
    ///     3. If the response is sent successfully, store the handshake.
    ///     4. Return the handshake, your address as seen by sender, and the version message.
    ///
    /// If the message is a Verack:
    ///
    ///     1. Get the existing handshake.
    ///     2. Mark the handshake as accepted.
    ///     3. Send a request for peers.
    ///     4. Return the accepted handshake and your address as seen by sender.
    ///
    /// TODO (howardwu): Fix the return type so it does not return Result<Option<T>>.
    #[inline]
    pub async fn connection_request(
        &self,
        block_height: u32,
        remote_address: SocketAddr,
        reader: TcpStream,
    ) -> Result<Option<(Channel, SocketAddr)>, NetworkError> {
        // Parse the inbound message into the message name and message bytes.
        let (channel, (message_name, message_bytes)) = match Channel::new(remote_address, reader) {
            // Read the next message from the channel.
            // Note this is a blocking operation.
            Ok(channel) => match channel.read().await {
                Ok(inbound_message) => (channel, inbound_message),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };

        // Handles a version message request.
        // Create and store a new handshake in the manager.
        match message_name {
            name if name == Version::name() => {
                // Deserialize the message bytes into a version message.
                let remote_version = match Version::deserialize(message_bytes) {
                    Ok(remote_version) => remote_version,
                    _ => return Ok(None),
                };

                let local_address = remote_version.receiver;

                // Create the remote address from the given peer address, and specified port from the version message.
                let remote_address = SocketAddr::new(remote_address.ip(), remote_version.sender.port());

                // Create the local version message.
                // TODO (raychu86): Establish a formal node version.
                let local_version = Version::new_with_rng(1u64, block_height, local_address, remote_address);

                debug_assert_eq!(local_address, local_version.sender);
                debug_assert_eq!(remote_address, local_version.receiver);

                // TODO (howardwu): Enable this sync logic if block height is lower than peer again.
                // if let Some(version) = version_message {
                //     // If our peer has a longer chain, send a sync message
                //     if version.height > environment.current_block_height().await {
                //         // Update the sync node if the sync_handler is Idle
                //         if let Ok(mut sync_handler) = sync_manager.try_lock() {
                //             if !sync_handler.is_syncing() {
                //                 sync_handler.sync_node_address = handshake.channel.address;
                //
                //                 if let Ok(block_locator_hashes) =
                //                     environment.storage_read().await.get_block_locator_hashes()
                //                 {
                //                     if let Err(err) =
                //                         handshake.channel.write(&GetSync::new(block_locator_hashes)).await
                //                     {
                //                         error!(
                //                             "Error sending GetSync message to {}, {}",
                //                             handshake.channel.address, err
                //                         );
                //                     }
                //                 }
                //             }
                //         }
                //     }
                // }

                // Connect to the remote address.
                let channel = channel.update_writer(remote_address).await?;

                // Write a verack response to the remote peer.
                channel
                    .write(&Verack::new(remote_version.nonce, local_address, remote_address))
                    .await?;
                // Write version request to the remote peer.
                channel.write(&local_version).await?;
                self.sender
                    .send(Response::ConnectingTo(local_version.receiver, local_version.nonce))
                    .await?;

                // Parse the inbound message into the message name and message bytes.
                let (message_name, message_bytes) = match channel.read().await {
                    Ok(inbound_message) => inbound_message,
                    _ => return Ok(None),
                };

                // Deserialize the message bytes into a verack message.
                let verack = match Verack::deserialize(message_bytes) {
                    Ok(verack) => verack,
                    _ => return Ok(None),
                };

                let local_address = verack.receiver;

                // TODO (howardwu): Check whether this remote address needs to
                //   be derive the same way as the version message case above
                //  (using a remote_address.ip() and address_sender.port()).
                let remote_address = verack.sender;

                // Store the new channel.
                self.channels.write().await.insert(remote_address, channel.clone());

                self.sender
                    .send(Response::ConnectedTo(remote_address, verack.nonce))
                    .await?;

                trace!("Established connection with {}", remote_address);

                return Ok(Some((channel, local_address)));
            }
            name if name == Verack::name() => {
                // Handles a verack message request.
                // Establish the channel with the remote peer.

                // Deserialize the message bytes into a verack message.
                let verack = match Verack::deserialize(message_bytes) {
                    Ok(verack) => verack,
                    _ => return Ok(None),
                };

                let local_address = verack.receiver;

                // TODO (howardwu): Check whether this remote address needs to
                //   be derive the same way as the version message case above
                //  (using a remote_address.ip() and address_sender.port()).
                let remote_address = verack.sender;

                // Store the new channel.
                self.channels.write().await.insert(remote_address, channel.clone());

                self.sender
                    .send(Response::ConnectedTo(remote_address, verack.nonce))
                    .await?;

                trace!("Established connection with {}", remote_address);

                return Ok(Some((channel, local_address)));
            }
            _ => warn!("Received a different message than Version/Verack when establishing a connection!"),
        }

        Ok(None)
    }
}
