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
    errors::ConnectError,
    external::{message::Message, message_types::*, Channel, MessageName},
    inbound::Response,
    Environment,
    NetworkError,
    Receiver,
    Sender,
};

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};

use parking_lot::RwLock;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex as AsyncMutex,
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
    receiver: Arc<AsyncMutex<Receiver>>,
    /// The map of remote addresses to their active read channels.
    channels: Arc<RwLock<Channels>>,
    /// A counter for the number of received responses the handler processes.
    receive_response_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that succeeded.
    receive_success_count: Arc<AtomicU64>,
    /// A counter for the number of received responses that failed.
    receive_failure_count: Arc<AtomicU64>,
}

impl Inbound {
    pub fn new(channels: Arc<RwLock<Channels>>) -> Self {
        // Initialize the sender and receiver.
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);

        Self {
            sender,
            receiver: Arc::new(AsyncMutex::new(receiver)),
            channels,
            receive_response_count: Default::default(),
            receive_success_count: Default::default(),
            receive_failure_count: Default::default(),
        }
    }

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
                        match inbound.connection_request(height, remote_address, channel).await {
                            Ok(channel) => {
                                let inbound = inbound.clone();
                                let channel = channel.clone();
                                tokio::spawn(async move {
                                    inbound.listen_for_messages(channel).await.unwrap();
                                });
                            }
                            Err(e) => error!("Failed to accept a connection: {}", e),
                        }
                    }
                    Err(e) => error!("Failed to accept a connection: {}", e),
                }
            }
        });

        Ok(())
    }

    pub async fn listen_for_messages(&self, channel: Channel) -> Result<(), NetworkError> {
        let mut failure_count = 0u8;
        let mut disconnect_from_peer = false;
        let mut channel = channel;
        let mut failure;
        loop {
            // Reset the failure indicator.
            failure = false;

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
            } else if name == GetPeers::name() {
                self.route(Response::GetPeers(channel.remote_address)).await;
            } else if name == Peers::name() {
                let message = Self::parse::<Peers>(&bytes)?;
                self.route(Response::Peers(channel.remote_address, message)).await;
            } else if name == Version::name() {
                let message = Self::parse::<Version>(&bytes)?;
                if let Err(err) = self.receive_version(message, channel.clone()).await {
                    error!("Failed to route response for a message\n{}", err);
                }
            } else if name == Verack::name() {
                let message = Self::parse::<Verack>(&bytes)?;
                self.route(Response::Verack(channel.remote_address, message)).await;
            } else if name == MessageName::from("disconnect") {
                self.route(Response::DisconnectFrom(channel.remote_address)).await;

                // TODO (howardwu): Remove this and rearchitect how disconnects are handled using the peer manager.
                // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
                warn!("Disconnecting from an unreliable peer");
                break;
            } else {
                debug!("Message name not recognized {:?}", name.to_string());
            }
        }

        Ok(())
    }

    /// Logs the failure and determines whether to disconnect from a peer.
    async fn handle_failure(
        failure: &mut bool,
        failure_count: &mut u8,
        disconnect_from_peer: &mut bool,
        error: ConnectError,
    ) {
        // Only increment failure_count if we haven't seen a failure yet.
        if !*failure {
            // Update the state to reflect a new failure.
            *failure = true;
            *failure_count += 1;
            error!("Connection error: {}", error);

            // Determine if we should disconnect.
            *disconnect_from_peer = error.is_fatal() || *failure_count >= 10;

            if *disconnect_from_peer {
                return;
            }
        } else {
            debug!("Connection errored again in the same loop (error message: {})", error);
        }

        // Sleep for 10 seconds
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    #[inline]
    fn parse<M: Message>(buffer: &[u8]) -> Result<M, NetworkError> {
        match M::deserialize(buffer) {
            Ok(message) => Ok(message),
            Err(error) => {
                error!("Failed to deserialize a message or {}B: {}", buffer.len(), error);
                Err(NetworkError::InboundDeserializationFailed)
            }
        }
    }

    #[inline]
    pub(crate) async fn route(&self, response: Response) {
        if let Err(err) = self.sender.send(response).await {
            error!("Failed to route a response for a message: {}", err);
        }
    }

    #[inline]
    pub(crate) fn receiver(&self) -> &AsyncMutex<Receiver> {
        &self.receiver
    }

    /// A connected peer has sent handshake request.
    /// If peer's block height is greater than ours, send a sync request.
    ///
    /// This method may seem redundant to handshake protocol functions but a peer can send additional
    /// Version messages if they want to update their ip address/port or want to share their chain height.
    async fn receive_version(&self, version: Version, channel: Channel) -> Result<(), NetworkError> {
        let remote_address = SocketAddr::new(channel.remote_address.ip(), version.sender.port());

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
        Ok(())
    }

    ///
    /// Receives a connection request with a given version message.
    ///
    /// Listens for the first message request from a remote peer.
    ///
    /// If the message is a Version:
    ///     1. Create a new handshake.
    ///     2. Send a handshake response.
    ///     3. If the response is sent successfully, store the handshake.
    ///     4. Return the handshake, your address as seen by sender, and the version message.
    ///
    /// If the message is a Verack:
    ///     1. Get the existing handshake.
    ///     2. Mark the handshake as accepted.
    ///     3. Send a request for peers.
    ///     4. Return the accepted handshake and your address as seen by sender.
    pub async fn connection_request(
        &self,
        block_height: u32,
        remote_address: SocketAddr,
        stream: TcpStream,
    ) -> Result<Channel, NetworkError> {
        // Register the new channel.
        let channel = Channel::new(remote_address, stream);

        // Read the next message from the channel.
        // Note: this is a blocking operation.
        let (message_name, message_bytes) = match channel.read().await {
            Ok(inbound_message) => inbound_message,
            _ => return Err(NetworkError::InvalidHandshake),
        };

        // Create and store a new handshake in the manager.
        if message_name == Version::name() {
            // Deserialize the message bytes into a version message.
            let remote_version = Version::deserialize(&message_bytes).map_err(|_| NetworkError::InvalidHandshake)?;

            // This is the node's address as seen by the peer.
            let local_address = remote_version.receiver;

            // Create the remote address from the given peer address, and specified port from the version message.
            let remote_address = SocketAddr::new(remote_address.ip(), remote_version.sender.port());

            // TODO: rename update_writer to update_address
            let channel = channel.update_writer(remote_address).await?;

            // Save the channel under the provided remote address
            self.channels.write().insert(remote_address, channel.clone());

            // TODO (raychu86): Establish a formal node version.
            let local_version = Version::new_with_rng(1u64, block_height, local_address, remote_address);

            // notify the server that the peer is being connected to
            self.sender
                .send(Response::ConnectingTo(remote_address, local_version.nonce))
                .await?;

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

            // Write a verack response to the remote peer.
            channel
                .write(&Verack::new(remote_version.nonce, local_address, remote_address))
                .await?;

            // Write a version request to the remote peer.
            channel.write(&local_version).await?;

            // Parse the inbound message into the message name and message bytes.
            let (_, message_bytes) = channel.read().await?;

            // Deserialize the message bytes into a verack message.
            let _verack = Verack::deserialize(&message_bytes).map_err(|_| NetworkError::InvalidHandshake)?;

            self.sender
                .send(Response::ConnectedTo(remote_address, local_version.nonce))
                .await?;

            Ok(channel)
        } else {
            Err(NetworkError::InvalidHandshake)
        }
    }
}
