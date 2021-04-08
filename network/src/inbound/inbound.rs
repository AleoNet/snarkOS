// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{errors::NetworkError, message::*, ConnReader, ConnWriter, Node, Receiver, Sender};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use parking_lot::{Mutex, RwLock};
use snarkvm_objects::Storage;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task,
};

/// The map of remote addresses to their active writers.
pub type Channels = HashMap<SocketAddr, Arc<ConnWriter>>;

/// A stateless component for handling inbound network traffic.
#[derive(Debug, Clone)]
pub struct Inbound {
    /// The producer for sending inbound messages to the server.
    pub(crate) sender: Sender,
    /// The consumer for receiving inbound messages to the server.
    receiver: Arc<Mutex<Option<Receiver>>>,
    /// The map of remote addresses to their active write channels.
    pub(crate) channels: Arc<RwLock<Channels>>,
}

impl Inbound {
    pub fn new(channels: Arc<RwLock<Channels>>) -> Self {
        // Initialize the sender and receiver.
        let (sender, receiver) = tokio::sync::mpsc::channel(1024);

        Self {
            sender,
            receiver: Arc::new(Mutex::new(Some(receiver))),
            channels,
        }
    }

    /// Logs the failure and determines whether to disconnect from a peer.
    fn handle_failure(
        failure: &mut bool,
        failure_count: &mut u8,
        disconnect_from_peer: &mut bool,
        error: NetworkError,
    ) {
        // Only increment failure_count if we haven't seen a failure yet.
        if !*failure {
            // Update the state to reflect a new failure.
            *failure = true;
            *failure_count += 1;
            error!("Network error: {}", error);

            // Determine if we should disconnect.
            *disconnect_from_peer = error.is_fatal() || *failure_count >= 10;

            if *disconnect_from_peer {
                debug!("Should disconnect from peer");
            }
        } else {
            debug!("A connection errored again in the same loop (error message: {})", error);
        }
    }

    #[inline]
    pub(crate) async fn route(&self, response: Message) {
        if let Err(err) = self.sender.send(response).await {
            error!("Failed to route a response for a message: {}", err);
        }
    }

    #[inline]
    pub(crate) fn take_receiver(&self) -> Receiver {
        self.receiver
            .lock()
            .take()
            .expect("The Inbound Receiver had already been taken!")
    }
}

impl<S: Storage + Send + Sync + 'static> Node<S> {
    pub async fn listen(&self, desired_address: Option<SocketAddr>) -> Result<(), NetworkError> {
        let (listener_address, listener) = if let Some(addr) = desired_address {
            let listener = TcpListener::bind(&addr).await?;
            (listener.local_addr()?, listener)
        } else {
            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let listener_address = listener.local_addr()?;
            (listener_address, listener)
        };
        self.environment.set_local_address(listener_address);
        info!("Node {:x} listening at {}", self.environment.name, listener_address);

        let node = self.clone();
        let listener_handle = task::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_address)) => {
                        info!("Got a connection request from {}", remote_address);

                        match node.connection_request(listener_address, remote_address, stream).await {
                            Ok((channel, mut reader)) => {
                                // update the remote address to be the peer's listening address
                                let remote_address = channel.addr;
                                // Save the channel under the provided remote address
                                node.inbound.channels.write().insert(remote_address, Arc::new(channel));

                                let node_clone = node.clone();
                                let conn_listening_task = tokio::spawn(async move {
                                    node_clone.listen_for_messages(&mut reader).await;
                                });

                                if let Ok(ref peer) = node.peer_book.read().get_peer(remote_address) {
                                    peer.register_task(conn_listening_task);
                                }
                            }
                            Err(e) => {
                                error!("Failed to accept a connection: {}", e);
                                let _ = node.disconnect_from_peer(remote_address);
                            }
                        }
                    }
                    Err(e) => error!("Failed to accept a connection: {}", e),
                }
            }
        });

        self.register_task(listener_handle);

        Ok(())
    }

    pub async fn listen_for_messages(&self, reader: &mut ConnReader) {
        let mut failure_count = 0u8;
        let mut disconnect_from_peer = false;
        let mut failure;

        loop {
            // Reset the failure indicator.
            failure = false;

            // Read the next message from the channel. This is a blocking operation.
            let message = match reader.read_message().await {
                Ok(message) => message,
                Err(error) => {
                    Inbound::handle_failure(&mut failure, &mut failure_count, &mut disconnect_from_peer, error);

                    // Determine if we should send a disconnect message.
                    match disconnect_from_peer {
                        true => {
                            // TODO (howardwu): Remove this and rearchitect how disconnects are handled using the peer manager.
                            // TODO (howardwu): Implement a handler so the node does not lose state of undetected disconnects.
                            warn!("Disconnecting from an unreliable peer");
                            let _ = self.disconnect_from_peer(reader.addr);
                            break; // the error has already been handled and reported
                        }
                        false => {
                            // Sleep for 10 seconds
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                }
            };

            // Messages are received by a single tokio MPSC receiver with
            // the message name, bytes, and associated channel.
            //
            // The oneshot sender lets the connection task know when the message is handled.

            self.inbound.route(message).await
        }
    }

    pub async fn process_incoming_messages(&self, receiver: &mut Receiver) -> Result<(), NetworkError> {
        let Message { direction, payload } = receiver.recv().await.ok_or(NetworkError::ReceiverFailedToParse)?;

        if self.environment.is_bootnode() && payload != Payload::GetPeers {
            // the bootstrapper nodes should ignore inbound messages other than GetPeers
            return Ok(());
        }

        let source = if let Direction::Inbound(addr) = direction {
            self.peer_book.read().update_last_seen(addr);
            Some(addr)
        } else {
            None
        };

        match payload {
            Payload::Transaction(transaction) => {
                if let Some(ref consensus) = self.consensus() {
                    let connected_peers = self.peer_book.read().connected_peers().clone();
                    consensus
                        .received_transaction(source.unwrap(), transaction, connected_peers)
                        .await?;
                }
            }
            Payload::Block(block) => {
                if let Some(ref consensus) = self.consensus() {
                    let connected_peers = self.peer_book.read().connected_peers().clone();
                    consensus
                        .received_block(source.unwrap(), block, Some(connected_peers))
                        .await?;
                }
            }
            Payload::SyncBlock(block) => {
                if let Some(ref consensus) = self.consensus() {
                    consensus.received_block(source.unwrap(), block, None).await?;
                    if self.peer_book.read().got_sync_block(source.unwrap()) {
                        consensus.finished_syncing_blocks();
                    }
                }
            }
            Payload::GetBlocks(hashes) => {
                if let Some(ref consensus) = self.consensus() {
                    if !consensus.is_syncing_blocks() {
                        consensus.received_get_blocks(source.unwrap(), hashes).await?;
                    }
                }
            }
            Payload::GetMemoryPool => {
                if let Some(ref consensus) = self.consensus() {
                    if !consensus.is_syncing_blocks() {
                        consensus.received_get_memory_pool(source.unwrap()).await?;
                    }
                }
            }
            Payload::MemoryPool(mempool) => {
                if let Some(ref consensus) = self.consensus() {
                    consensus.received_memory_pool(mempool)?;
                }
            }
            Payload::GetSync(getsync) => {
                if let Some(ref consensus) = self.consensus() {
                    if !consensus.is_syncing_blocks() {
                        consensus.received_get_sync(source.unwrap(), getsync).await?;
                    }
                }
            }
            Payload::Sync(sync) => {
                if let Some(ref consensus) = self.consensus() {
                    self.peer_book.read().expecting_sync_blocks(source.unwrap(), sync.len());
                    consensus.received_sync(source.unwrap(), sync).await;
                }
            }
            Payload::GetPeers => {
                self.send_peers(source.unwrap()).await;
            }
            Payload::Peers(peers) => {
                self.process_inbound_peers(peers);
            }
            Payload::Ping(block_height) => {
                self.outbound
                    .send_request(Message::new(Direction::Outbound(source.unwrap()), Payload::Pong))
                    .await;

                if let Some(ref consensus) = self.consensus() {
                    if block_height > consensus.current_block_height() + 1
                        && consensus.should_sync_blocks()
                        && !self.peer_book.read().is_syncing_blocks(source.unwrap())
                    {
                        consensus.register_block_sync_attempt();
                        trace!("Attempting to sync with {}", source.unwrap());
                        consensus.update_blocks(source.unwrap()).await;
                    } else {
                        consensus.finished_syncing_blocks();
                    }
                }
            }
            Payload::Pong => {
                self.peer_book.read().received_pong(source.unwrap());
            }
            Payload::Unknown => {
                warn!("Unknown payload received; this could indicate that the client you're using is out-of-date");
            }
        }

        Ok(())
    }

    ///
    /// Handles an incoming connection request, performing a secure handshake and establishing packet encryption.
    ///
    pub async fn connection_request(
        &self,
        listener_address: SocketAddr,
        remote_address: SocketAddr,
        stream: TcpStream,
    ) -> Result<(ConnWriter, ConnReader), NetworkError> {
        self.peer_book.write().set_connecting(remote_address)?;

        let (mut reader, mut writer) = stream.into_split();

        let builder = snow::Builder::with_resolver(
            crate::HANDSHAKE_PATTERN
                .parse()
                .expect("Invalid noise handshake pattern!"),
            Box::new(snow::resolvers::SodiumResolver),
        );
        let static_key = builder.generate_keypair()?.private;
        let noise_builder = builder.local_private_key(&static_key).psk(3, crate::HANDSHAKE_PSK);
        let mut noise = noise_builder.build_responder()?;
        let mut buffer: Box<[u8]> = vec![0u8; crate::MAX_MESSAGE_SIZE].into();
        let mut buf = [0u8; crate::NOISE_BUF_LEN]; // a temporary intermediate buffer to decrypt from

        // <- e
        reader.read_exact(&mut buf[..1]).await?;
        let len = buf[0] as usize;
        if len == 0 {
            return Err(NetworkError::InvalidHandshake);
        }
        let len = reader.read_exact(&mut buf[..len]).await?;
        noise.read_message(&buf[..len], &mut buffer)?;
        trace!("received e (XX handshake part 1/3)");

        // -> e, ee, s, es
        let own_version = Version::serialize(&Version::new(1u64, listener_address.port())).unwrap(); // TODO (raychu86): Establish a formal node version.
        let len = noise.write_message(&own_version, &mut buffer)?;
        writer.write_all(&[len as u8]).await?;
        writer.write_all(&buffer[..len]).await?;
        trace!("sent e, ee, s, es (XX handshake part 2/3)");

        // <- s, se, psk
        reader.read_exact(&mut buf[..1]).await?;
        let len = buf[0] as usize;
        if len == 0 {
            return Err(NetworkError::InvalidHandshake);
        }
        let len = reader.read_exact(&mut buf[..len]).await?;
        let len = noise.read_message(&buf[..len], &mut buffer)?;
        let peer_version = Version::deserialize(&buffer[..len])?;
        trace!("received s, se, psk (XX handshake part 3/3)");

        // the remote listening address
        let remote_listener = SocketAddr::from((remote_address.ip(), peer_version.listening_port));

        self.peer_book
            .write()
            .set_connected(remote_address, Some(remote_listener))?;

        let noise = Arc::new(Mutex::new(noise.into_transport_mode()?));
        let reader = ConnReader::new(remote_listener, reader, buffer.clone(), Arc::clone(&noise));
        let writer = ConnWriter::new(remote_listener, writer, buffer, noise);

        Ok((writer, reader))
    }
}
