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

use crate::{errors::NetworkError, message::*, stats, Cache, ConnReader, ConnWriter, Node, Receiver, Sender, State};

use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

use parking_lot::Mutex;
use snarkvm_objects::Storage;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, error::TrySendError},
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
}

impl Default for Inbound {
    fn default() -> Self {
        // Initialize the sender and receiver.
        let (sender, receiver) = tokio::sync::mpsc::channel(crate::INBOUND_CHANNEL_DEPTH);

        Self {
            sender,
            receiver: Mutex::new(Some(receiver)),
        }
    }
}

impl Inbound {
    #[inline]
    pub(crate) fn take_receiver(&self) -> Receiver {
        self.receiver
            .lock()
            .take()
            .expect("The Inbound Receiver had already been taken!")
    }
}

impl<S: Storage + Send + Sync + 'static> Node<S> {
    /// This method handles new inbound connection requests.
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
        info!("Initializing listener for node ({:x})", self.id);

        let node_clone = self.clone();
        let listener_handle = task::spawn(async move {
            info!("Listening for nodes at {}", listener_address);

            loop {
                match listener.accept().await {
                    Ok((stream, remote_address)) => {
                        info!("Got a connection request from {}", remote_address);

                        if !node_clone.can_connect() {
                            metrics::increment_counter!(stats::CONNECTIONS_ALL_REJECTED);
                            continue;
                        }

                        let node = node_clone.clone();
                        task::spawn(async move {
                            // Wait a maximum timeout limit for a connection request.
                            let handshake_result = tokio::time::timeout(
                                Duration::from_secs(crate::HANDSHAKE_PEER_TIMEOUT_SECS as u64),
                                node.connection_request(listener_address, remote_address, stream),
                            )
                            .await;

                            match handshake_result {
                                Ok(Ok((mut writer, mut reader, remote_listener))) => {
                                    // Create a channel dedicated to sending messages to the connection.
                                    let (sender, receiver) = channel(crate::OUTBOUND_CHANNEL_DEPTH);

                                    // Listen for inbound messages.
                                    let node_clone = node.clone();
                                    let peer_reading_task = tokio::spawn(async move {
                                        node_clone.listen_for_inbound_messages(&mut reader).await;
                                    });

                                    // Listen for outbound messages.
                                    let node_clone = node.clone();
                                    let peer_writing_task = tokio::spawn(async move {
                                        node_clone.listen_for_outbound_messages(receiver, &mut writer).await;
                                    });

                                    // Save the channel under the provided remote address.
                                    node.outbound.channels.write().insert(remote_listener, sender);

                                    // Finally, mark the peer as connected.
                                    node.peer_book.set_connected(remote_address, Some(remote_listener));

                                    trace!("Connected to {} (listener: {})", remote_address, remote_listener);

                                    // Immediately send a ping to provide the peer with our block height.
                                    node.send_ping(remote_listener);

                                    if let Ok(ref peer) = node.peer_book.get_peer(remote_listener) {
                                        peer.register_task(peer_reading_task, true);
                                        peer.register_task(peer_writing_task, false);
                                    } else {
                                        // If the related peer is not found, it means it's already been dropped.
                                        peer_reading_task.abort();
                                        peer_writing_task.abort();
                                    }
                                }
                                Ok(Err(e)) => {
                                    error!("Failed to accept a connection request: {}", e);
                                    node.disconnect_from_peer(remote_address);
                                    metrics::increment_counter!(stats::HANDSHAKES_FAILURES_RESP);
                                }
                                Err(_) => {
                                    error!("Failed to accept a connection request: the handshake timed out");
                                    node.disconnect_from_peer(remote_address);
                                    metrics::increment_counter!(stats::HANDSHAKES_TIMEOUTS_RESP);
                                }
                            }
                        });

                        // add a tiny delay to avoid connecting above the limit
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(e) => error!("Failed to accept a connection: {}", e),
                }
                metrics::increment_counter!(stats::CONNECTIONS_ALL_ACCEPTED);
            }
        });

        self.register_task(listener_handle);

        Ok(())
    }

    /// This method handles new inbound messages from a single connected node.
    pub async fn listen_for_inbound_messages(&self, reader: &mut ConnReader) {
        let mut failure_count = 0u8;

        loop {
            // Read the next message from the channel.
            let message = match reader.read_message().await {
                Ok(message) => message,
                Err(error) => {
                    // Log the failure and increment the failure count.
                    error!("Unable to read message from {}: {}", reader.addr, error);
                    failure_count += 1;

                    // Determine if we should disconnect.
                    let disconnect_from_peer = error.is_fatal() || failure_count >= 10;

                    // Determine if we should send a disconnect message.
                    match disconnect_from_peer {
                        true => {
                            warn!("Disconnecting from {} (unreliable)", reader.addr);
                            self.disconnect_from_peer(reader.addr);
                            // The error has been handled and reported, we may now safely break.
                            break;
                        }
                        false => {
                            // Sleep for 10 seconds
                            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                            continue;
                        }
                    }
                }
            };

            // Route the message to the inbound handler of this node.
            {
                // Handle Ping/Pong messages immediately in order not to skew latency calculation.
                match &message.payload {
                    Payload::Ping(..) => {
                        self.send_request(Message::new(Direction::Outbound(reader.addr), Payload::Pong));
                    }
                    Payload::Pong => {
                        self.peer_book.received_pong(reader.addr);
                    }
                    _ => {}
                }

                // Messages are queued in a single tokio MPSC receiver.
                self.route(message)
            }
        }
    }

    pub async fn process_incoming_messages(
        &self,
        receiver: &mut Receiver,
        cache: &mut Cache,
    ) -> Result<(), NetworkError> {
        let Message { direction, payload } = receiver.recv().await.ok_or(NetworkError::ReceiverFailedToParse)?;

        metrics::decrement_gauge!(stats::QUEUES_INBOUND, 1.0);

        let source = if let Direction::Inbound(addr) = direction {
            addr
        } else {
            unreachable!("All messages processed sent to the inbound receiver are Inbound");
        };

        self.peer_book.register_message(source);

        // Check if the message hasn't already been processed recently if it's a `Block`.
        // The node should also reject them while syncing, as it is bound to receive them later.
        if matches!(payload, Payload::Block(..)) && (self.state() == State::Syncing || cache.contains(&payload)) {
            return Ok(());
        }

        match payload {
            Payload::Transaction(transaction) => {
                metrics::increment_counter!(stats::INBOUND_TRANSACTIONS);

                if let Some(ref sync) = self.sync() {
                    sync.received_memory_pool_transaction(source, transaction)?;
                }
            }
            Payload::Block(block) => {
                metrics::increment_counter!(stats::INBOUND_BLOCKS);

                if let Some(ref sync) = self.sync() {
                    sync.received_block(source, block, true)?;
                }
            }
            Payload::SyncBlock(block) => {
                metrics::increment_counter!(stats::INBOUND_SYNCBLOCKS);

                if let Some(ref sync) = self.sync() {
                    sync.received_block(source, block, false)?;

                    // Update the peer and possibly finish the sync process.
                    if self.peer_book.got_sync_block(source) {
                        sync.finished_syncing_blocks();
                    }
                }
            }
            Payload::GetBlocks(hashes) => {
                metrics::increment_counter!(stats::INBOUND_GETBLOCKS);

                if let Some(ref sync) = self.sync() {
                    sync.received_get_blocks(source, hashes)?;
                }
            }
            Payload::GetMemoryPool => {
                metrics::increment_counter!(stats::INBOUND_GETMEMORYPOOL);

                if let Some(ref sync) = self.sync() {
                    sync.received_get_memory_pool(source);
                }
            }
            Payload::MemoryPool(mempool) => {
                metrics::increment_counter!(stats::INBOUND_MEMORYPOOL);

                if let Some(ref sync) = self.sync() {
                    sync.received_memory_pool(mempool)?;
                }
            }
            Payload::GetSync(getsync) => {
                metrics::increment_counter!(stats::INBOUND_GETSYNC);

                if let Some(ref sync) = self.sync() {
                    sync.received_get_sync(source, getsync)?;
                }
            }
            Payload::Sync(sync) => {
                metrics::increment_counter!(stats::INBOUND_SYNCS);

                if let Some(ref sync_handler) = self.sync() {
                    if sync.is_empty() {
                        // An empty `Sync` is unexpected, as `GetSync` requests are only
                        // sent to peers that declare a greater block height.
                        self.peer_book.register_failure(source);
                        warn!("{} doesn't have sync blocks to share", source);
                    } else if self.peer_book.expecting_sync_blocks(source, sync.len()) {
                        trace!("Received {} sync block hashes from {}", sync.len(), source);
                        sync_handler.received_sync(source, sync);
                    }
                }
            }
            Payload::GetPeers => {
                metrics::increment_counter!(stats::INBOUND_GETPEERS);

                self.send_peers(source);
            }
            Payload::Peers(peers) => {
                metrics::increment_counter!(stats::INBOUND_PEERS);

                self.process_inbound_peers(peers);
            }
            Payload::Ping(block_height) => {
                metrics::increment_counter!(stats::INBOUND_PINGS);

                self.peer_book.received_ping(source, block_height);
            }
            Payload::Pong => {
                metrics::increment_counter!(stats::INBOUND_PONGS);
                // Skip as this case is already handled with priority in Inbound::listen_for_messages
            }
            Payload::Unknown => {
                metrics::increment_counter!(stats::INBOUND_UNKNOWN);
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
    ) -> Result<(ConnWriter, ConnReader, SocketAddr), NetworkError> {
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
        let own_version =
            Version::serialize(&Version::new(crate::PROTOCOL_VERSION, listener_address.port(), self.id)).unwrap();
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

        if peer_version.node_id == self.id {
            return Err(NetworkError::SelfConnectAttempt);
        }
        if peer_version.version != crate::PROTOCOL_VERSION {
            return Err(NetworkError::InvalidHandshake);
        }

        metrics::increment_counter!(stats::HANDSHAKES_SUCCESSES_RESP);

        // the remote listening address
        let remote_listener = SocketAddr::from((remote_address.ip(), peer_version.listening_port));

        let noise = Arc::new(Mutex::new(noise.into_transport_mode()?));
        let reader = ConnReader::new(remote_listener, reader, buffer.clone(), Arc::clone(&noise));
        let writer = ConnWriter::new(remote_listener, writer, buffer, noise);

        Ok((writer, reader, remote_listener))
    }

    #[inline]
    pub(crate) fn route(&self, response: Message) {
        match self.inbound.sender.try_send(response) {
            Err(TrySendError::Full(msg)) => {
                metrics::increment_counter!(stats::INBOUND_ALL_FAILURES);
                error!("Failed to route a {}: the inbound channel is full", msg);
            }
            Err(TrySendError::Closed(msg)) => {
                // TODO: this shouldn't happen, but is critical if it does
                error!("Failed to route a {}: the inbound channel is closed", msg);
            }
            Ok(_) => {
                metrics::increment_gauge!(stats::QUEUES_INBOUND, 1.0);
            }
        }
    }
}
