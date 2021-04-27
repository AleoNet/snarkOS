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

use crate::{errors::NetworkError, message::*, ConnReader, ConnWriter, Node, Receiver, Sender, State};

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

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
#[derive(Debug)]
pub struct Inbound {
    /// The producer for sending inbound messages to the server.
    pub(crate) sender: Sender,
    /// The consumer for receiving inbound messages to the server.
    receiver: Mutex<Option<Receiver>>,
    /// The map of remote addresses to their active write channels.
    pub(crate) channels: Arc<RwLock<Channels>>,
}

impl Inbound {
    pub fn new(channels: Arc<RwLock<Channels>>) -> Self {
        // Initialize the sender and receiver.
        let (sender, receiver) = tokio::sync::mpsc::channel(64 * 1024);

        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
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
    pub async fn listen(&self) -> Result<(), NetworkError> {
        let (listener_address, listener) = if let Some(addr) = self.config.desired_address {
            let listener = TcpListener::bind(&addr).await?;
            (listener.local_addr()?, listener)
        } else {
            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let listener_address = listener.local_addr()?;
            (listener_address, listener)
        };
        self.set_local_address(listener_address);
        info!("This node ({:x}) is listening at {}", self.name, listener_address);

        let node = self.clone();
        let listener_handle = task::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, remote_address)) => {
                        info!("Got a connection request from {}", remote_address);

                        let handshake_result = tokio::time::timeout(
                            Duration::from_secs(crate::HANDSHAKE_TIME_LIMIT_SECS as u64),
                            node.connection_request(listener_address, remote_address, stream),
                        )
                        .await;

                        match handshake_result {
                            Ok(Ok((channel, mut reader))) => {
                                // update the remote address to be the peer's listening address
                                let remote_address = channel.addr;
                                // Save the channel under the provided remote address
                                node.inbound.channels.write().insert(remote_address, Arc::new(channel));

                                let node_clone = node.clone();
                                let conn_listening_task = tokio::spawn(async move {
                                    node_clone.listen_for_messages(&mut reader).await;
                                });

                                trace!("Connected to {}", remote_address);

                                // immediately send a ping to provide the peer with our block height
                                node.send_ping(remote_address).await;

                                if let Ok(ref peer) = node.peer_book.get_peer(remote_address) {
                                    peer.register_task(conn_listening_task);
                                } else {
                                    // if the related peer is not found, it means it's already been dropped
                                    conn_listening_task.abort();
                                }
                            }
                            Ok(Err(e)) => {
                                error!("Failed to accept a connection: {}", e);
                                let _ = node.disconnect_from_peer(remote_address);
                            }
                            Err(_) => {
                                error!("Failed to accept a connection: the handshake timed out");
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

            // Read the next message from the channel.
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

            // Handle Ping/Pong messages immediately in order not to skew latency calculation.
            match &message.payload {
                Payload::Ping(..) => {
                    self.outbound
                        .send_request(Message::new(Direction::Outbound(reader.addr), Payload::Pong))
                        .await;
                }
                Payload::Pong => {
                    self.peer_book.received_pong(reader.addr);
                }
                _ => {}
            }

            // Messages are queued in a single tokio MPSC receiver.
            self.inbound.route(message).await
        }
    }

    pub async fn process_incoming_messages(&self, receiver: &mut Receiver) -> Result<(), NetworkError> {
        let Message { direction, payload } = receiver.recv().await.ok_or(NetworkError::ReceiverFailedToParse)?;

        if self.config.is_bootnode() && payload != Payload::GetPeers {
            // the bootstrapper nodes should ignore inbound messages other than GetPeers
            return Ok(());
        }

        let source = if let Direction::Inbound(addr) = direction {
            self.peer_book.update_last_seen(addr);
            addr
        } else {
            unreachable!("All messages processed sent to the inbound receiver are Inbound");
        };

        match payload {
            Payload::Transaction(transaction) => {
                if let Some(ref consensus) = self.consensus() {
                    consensus.received_transaction(source, transaction).await?;
                }
            }
            Payload::Block(block) => {
                if let Some(ref consensus) = self.consensus() {
                    consensus.received_block(source, block, true).await?;
                }
            }
            Payload::SyncBlock(block) => {
                if let Some(ref consensus) = self.consensus() {
                    consensus.received_block(source, block, false).await?;

                    // update the peer and possibly finish the sync process
                    if self.peer_book.got_sync_block(source) {
                        consensus.finished_syncing_blocks();
                    } else {
                        // since we confirmed that the block is a valid sync block
                        // and we're expecting more blocks from the peer, we can set
                        // the node's status to Syncing
                        consensus.node().set_state(State::Syncing);
                    }
                }
            }
            Payload::GetBlocks(hashes) => {
                if let Some(ref consensus) = self.consensus() {
                    if !consensus.is_syncing_blocks() {
                        consensus.received_get_blocks(source, hashes).await?;
                    }
                }
            }
            Payload::GetMemoryPool => {
                if let Some(ref consensus) = self.consensus() {
                    if !consensus.is_syncing_blocks() {
                        consensus.received_get_memory_pool(source).await?;
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
                        consensus.received_get_sync(source, getsync).await?;
                    }
                }
            }
            Payload::Sync(sync) => {
                if let Some(ref consensus) = self.consensus() {
                    if !sync.is_empty() && self.peer_book.expecting_sync_blocks(source, sync.len()) {
                        consensus.received_sync(source, sync).await;
                    }
                }
            }
            Payload::GetPeers => {
                self.send_peers(source).await;
            }
            Payload::Peers(peers) => {
                self.process_inbound_peers(peers);
            }
            Payload::Ping(block_height) => {
                if let Some(ref consensus) = self.consensus() {
                    if block_height > consensus.current_block_height() + 1 {
                        // if the node is syncing, check if that sync attempt hasn't expired
                        if !consensus.is_syncing_blocks() || consensus.has_block_sync_expired() {
                            // cancel any possibly ongoing sync attempts
                            self.set_state(State::Idle);
                            self.peer_book.cancel_any_unfinished_syncing();

                            // begin a new sync attempt
                            consensus.register_block_sync_attempt(source);
                            consensus.update_blocks(source).await;
                        }
                    }
                }
            }
            Payload::Pong => {
                // already handled with priority in Inbound::listen_for_messages
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
        self.peer_book.set_connecting(remote_address)?;

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
        trace!("received e (XX handshake part 1/3) from {}", remote_address);

        // -> e, ee, s, es
        let own_version = Version::serialize(&Version::new(1u64, listener_address.port())).unwrap(); // TODO (raychu86): Establish a formal node version.
        let len = noise.write_message(&own_version, &mut buffer)?;
        writer.write_all(&[len as u8]).await?;
        writer.write_all(&buffer[..len]).await?;
        trace!("sent e, ee, s, es (XX handshake part 2/3) to {}", remote_address);

        // <- s, se, psk
        reader.read_exact(&mut buf[..1]).await?;
        let len = buf[0] as usize;
        if len == 0 {
            return Err(NetworkError::InvalidHandshake);
        }
        let len = reader.read_exact(&mut buf[..len]).await?;
        let len = noise.read_message(&buf[..len], &mut buffer)?;
        let peer_version = Version::deserialize(&buffer[..len])?;
        trace!("received s, se, psk (XX handshake part 3/3) from {}", remote_address);

        // the remote listening address
        let remote_listener = SocketAddr::from((remote_address.ip(), peer_version.listening_port));

        self.peer_book.set_connected(remote_address, Some(remote_listener))?;

        let noise = Arc::new(Mutex::new(noise.into_transport_mode()?));
        let reader = ConnReader::new(remote_listener, reader, buffer.clone(), Arc::clone(&noise));
        let writer = ConnWriter::new(remote_listener, writer, buffer, noise);

        Ok((writer, reader))
    }
}
