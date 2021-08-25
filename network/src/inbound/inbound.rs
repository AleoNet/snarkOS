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

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

#[cfg(not(feature = "test"))]
use tokio::runtime;
use tokio::{
    net::TcpListener,
    sync::{mpsc::error::TrySendError, Mutex},
    task,
};

use snarkos_metrics::{self as metrics, connections, inbound, queues};

use crate::{errors::NetworkError, message::*, Node, Receiver, Sender};

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
    pub(crate) async fn take_receiver(&self) -> Receiver {
        self.receiver
            .lock()
            .await
            .take()
            .expect("The Inbound Receiver had already been taken!")
    }
}

impl Node {
    /// This method handles new inbound connection requests.
    pub async fn listen(&self) -> Result<(), NetworkError> {
        let listener = TcpListener::bind(&self.config.desired_address).await?;
        let own_listener_address = listener.local_addr()?;

        self.set_local_address(own_listener_address);
        info!("Initializing listener for node ({:x})", self.id);

        let node_clone = self.clone();
        let listener_handle = task::spawn(async move {
            info!("Listening for nodes at {}", own_listener_address);

            loop {
                match listener.accept().await {
                    Ok((stream, remote_address)) => {
                        if !node_clone.can_connect() {
                            continue;
                        }
                        let node_clone = node_clone.clone();
                        tokio::spawn(async move {
                            match node_clone
                                .peer_book
                                .receive_connection(node_clone.clone(), remote_address, stream)
                            {
                                Ok(_) => (),
                                Err(e) => {
                                    error!("Failed to receive a connection: {}", e);
                                }
                            }
                        });

                        // add a tiny delay to avoid connecting above the limit
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(e) => error!("Failed to accept a connection: {}", e),
                }
                metrics::increment_counter!(connections::ALL_ACCEPTED);
            }
        });

        self.register_task(listener_handle);

        Ok(())
    }

    #[cfg(not(feature = "test"))]
    pub fn process_incoming_messages(&self, receiver: &mut Receiver, rt_handle: &runtime::Handle) {
        while let Some((time_received, Message { direction, payload })) = receiver.blocking_recv() {
            metrics::decrement_gauge!(queues::INBOUND, 1.0);
            metrics::increment_counter!(inbound::ALL_SUCCESSES);

            let source = if let Direction::Inbound(addr) = direction {
                addr
            } else {
                unreachable!("All messages processed sent to the inbound receiver are Inbound");
            };

            let node = self.clone();
            rt_handle.spawn(async move { node.process_incoming_message(payload, source, time_received).await });
        }
    }

    #[cfg(feature = "test")]
    pub async fn process_incoming_messages(&self, receiver: &mut Receiver) {
        while let Some((time_received, Message { direction, payload })) = receiver.recv().await {
            metrics::decrement_gauge!(queues::INBOUND, 1.0);
            metrics::increment_counter!(inbound::ALL_SUCCESSES);

            let source = if let Direction::Inbound(addr) = direction {
                addr
            } else {
                unreachable!("All messages processed sent to the inbound receiver are Inbound");
            };

            self.process_incoming_message(payload, source, time_received).await;
        }
    }

    pub async fn process_incoming_message(&self, payload: Payload, source: SocketAddr, time_received: Option<Instant>) {
        match payload {
            Payload::Transaction(transaction) => {
                metrics::increment_counter!(inbound::TRANSACTIONS);

                if self.sync().is_some() {
                    if let Err(e) = self.received_memory_pool_transaction(source, transaction).await {
                        warn!("Received an invalid transaction from a peer: {}", e);
                        if let Some(peer) = self.peer_book.get_peer_handle(source) {
                            peer.fail().await;
                        }
                    }
                }
            }
            Payload::Block(block, height) => {
                // The BLOCKS metric was already updated during the block dedup cache lookup.

                if self.sync().is_some() {
                    if let Err(e) = self.received_block(source, block, height, true).await {
                        warn!("Received an invalid block from a peer: {}", e);
                        if let Some(peer) = self.peer_book.get_peer_handle(source) {
                            peer.fail().await;
                        }
                    }
                }
            }
            Payload::SyncBlock(block, height) => {
                metrics::increment_counter!(inbound::SYNCBLOCKS);

                if self.sync().is_some() {
                    if let Err(e) = self.received_block(source, block, height, false).await {
                        warn!("Received an invalid block from a peer: {}", e);
                        if let Some(peer) = self.peer_book.get_peer_handle(source) {
                            peer.fail().await;
                        }
                        return;
                    };

                    // Update the peer and possibly finish the sync process.
                    if let Some(peer) = self.peer_book.get_peer_handle(source) {
                        peer.got_sync_block().await;
                    }
                }
            }
            Payload::GetBlocks(hashes) => {
                metrics::increment_counter!(inbound::GETBLOCKS);

                if self.sync().is_some() {
                    let hashes = hashes.into_iter().map(|x| x.0.into()).collect();

                    if let Err(e) = self.received_get_blocks(source, hashes, time_received).await {
                        warn!("failed to send sync blocks to peer: {:?}", e);
                    }
                }
            }
            Payload::GetMemoryPool => {
                metrics::increment_counter!(inbound::GETMEMORYPOOL);

                if self.sync().is_some() {
                    if let Err(e) = self.received_get_memory_pool(source, time_received).await {
                        warn!("Failed to procure the memory pool for a peer: {:?}", e);
                    }
                }
            }
            Payload::MemoryPool(mempool) => {
                metrics::increment_counter!(inbound::MEMORYPOOL);

                if self.sync().is_some() {
                    if let Err(e) = self.received_memory_pool(mempool).await {
                        warn!("Received an invalid memory pool from a peer: {}", e);
                        if let Some(peer) = self.peer_book.get_peer_handle(source) {
                            peer.fail().await;
                        }
                    }
                }
            }
            Payload::GetSync(getsync) => {
                metrics::increment_counter!(inbound::GETSYNC);

                if self.sync().is_some() {
                    let getsync = getsync.into_iter().map(|x| x.0.into()).collect();
                    if let Err(e) = self.received_get_sync(source, getsync, time_received).await {
                        warn!("Failed to procure sync blocks for a peer: {}", e);
                    }
                }
            }
            Payload::Sync(sync) => {
                metrics::increment_counter!(inbound::SYNCS);

                if self.sync().is_some() {
                    if sync.is_empty() {
                        // An empty `Sync` is unexpected, as `GetSync` requests are only
                        // sent to peers that declare a greater block height.
                        warn!("{} doesn't have sync blocks to share", source);
                        if let Some(peer) = self.peer_book.get_peer_handle(source) {
                            peer.fail().await;
                        }
                    } else {
                        trace!("Received {} sync block hashes from {}", sync.len(), source);
                        self.received_sync(source, sync).await;
                    }
                }
            }
            Payload::GetPeers => {
                metrics::increment_counter!(inbound::GETPEERS);
                self.send_peers(source, time_received).await;
            }
            Payload::Peers(peers) => {
                metrics::increment_counter!(inbound::PEERS);
                self.process_inbound_peers(source, peers).await;
            }
            Payload::Ping(_) | Payload::Pong => {
                // Skip as this case is already handled with priority in inbound_handler
                unreachable!()
            }
            Payload::Unknown => {
                metrics::increment_counter!(inbound::UNKNOWN);
                warn!("Unknown payload received; this could indicate that the client you're using is out-of-date");
            }
        }
    }

    #[inline]
    pub(crate) fn route(&self, time_received: Option<Instant>, response: Message) {
        match self.inbound.sender.try_send((time_received, response)) {
            Err(TrySendError::Full((_, msg))) => {
                metrics::increment_counter!(inbound::ALL_FAILURES);
                error!("Failed to route a {}: the inbound channel is full", msg);
            }
            Err(TrySendError::Closed((_, msg))) => {
                // TODO: this shouldn't happen, but is critical if it does
                error!("Failed to route a {}: the inbound channel is closed", msg);
            }
            Ok(_) => {
                metrics::increment_gauge!(queues::INBOUND, 1.0);
            }
        }
    }
}
