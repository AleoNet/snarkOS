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
    external::{
        message_types::{GetMemoryPool, GetPeers, Version},
        protocol::sync::SyncState,
    },
    Server,
};

use chrono::{Duration as ChronoDuration, Utc};
use std::time::Duration;
use tokio::{task, time::delay_for};

impl Server {
    /// Manages the number of active connections according to the connection frequency.
    /// 1. Get more connected peers if we are under the minimum number specified by the network context.
    ///     1.1 Ask our connected peers for their peers.
    ///     1.2 Ask our gossiped peers to handshake and become connected.
    /// 2. Maintain connected peers by sending ping messages.
    /// 3. Purge peers that have not responded in connection_frequency x 5 seconds.
    /// 4. Reselect a sync node if we purged it.
    /// 5. Update our memory pool every connection_frequency x memory_pool_interval seconds.
    /// All errors encountered by the connection handler will be logged to the console but will not stop the thread.
    pub async fn connection_handler(&self) {
        let context = self.context.clone();
        let memory_pool_lock = self.memory_pool_lock.clone();
        let sync_handler_lock = self.sync_handler_lock.clone();
        let storage = self.storage.clone();
        let connection_frequency = self.connection_frequency;

        // Start a separate thread for the handler.
        task::spawn(async move {
            let mut interval_ticker: u8 = 0;

            loop {
                {
                    // There are a lot of potentially expensive and blocking operations here.
                    // Let's wait for connection_frequency seconds before starting each loop.
                    delay_for(Duration::from_millis(connection_frequency)).await;

                    // Remove the local_address from the peer book
                    // if the node somehow discovered itself as a peer.
                    let local_address = *context.local_address.read().await;
                    {
                        // Acquire the peer book write lock.
                        let mut peer_book = context.peer_book.write().await;
                        peer_book.forget_peer(local_address);
                        drop(peer_book);
                    }

                    let (num_connected_peers, connected_peers, disconnected_peers) = {
                        let peer_book = context.peer_book.read().await;
                        let num_connected_peers = peer_book.num_connected();
                        let connected_peers = peer_book.get_connected().clone();
                        let disconnected_peers = peer_book.get_disconnected().clone();
                        drop(peer_book);
                        (num_connected_peers, connected_peers, disconnected_peers)
                    };

                    let connections = context.connections.read().await;
                    let pings = &mut context.pings.write().await;

                    // If the node is connected to less peers than the minimum required,
                    // broadcast a `GetPeers` message to request for more peers.
                    if num_connected_peers < context.min_peers {
                        // Send a `GetPeers` message to every peer the node is connected to.
                        for (remote_address, _) in &connected_peers {
                            match connections.get(&remote_address) {
                                // Disconnect from the peer if the message fails to send.
                                Some(channel) => {
                                    if let Err(_) = channel.write(&GetPeers).await {
                                        // Acquire the peer book write lock.
                                        let mut peer_book = context.peer_book.write().await;
                                        peer_book.disconnected_peer(&remote_address);
                                        drop(peer_book);
                                    }
                                }
                                // Disconnect from the peer if the channel is not active.
                                None => {
                                    // Acquire the peer book write lock.
                                    let mut peer_book = context.peer_book.write().await;
                                    peer_book.disconnected_peer(&remote_address);
                                    drop(peer_book);
                                }
                            }
                        }

                        // Attempt a handshake with every disconnected peer.
                        for (remote_address, _) in disconnected_peers {
                            let new_context = context.clone();
                            let latest_block_height = storage.get_latest_block_height();

                            // Create a non-blocking handshake request.
                            task::spawn(async move {
                                // TODO (raychu86) Establish a formal node version
                                let version = Version::new(1u64, latest_block_height, remote_address, local_address);

                                // Acquire the handshake write lock.
                                let mut handshakes = new_context.handshakes.write().await;
                                // Attempt a handshake with the remote address.
                                if let Err(_) = handshakes.send_request(&version).await {
                                    debug!("Tried connecting to disconnected peer {} and failed", remote_address);
                                }
                                drop(handshakes)
                            });
                        }
                    }

                    // Broadcast a `Ping` request to every connected peers
                    // that the node hasn't heard from in a while.
                    for (remote_address, peer_info) in &connected_peers {
                        // Calculate the time since we last saw the peer.
                        let elapsed_in_millis = (Utc::now() - *peer_info.last_seen()).num_milliseconds();
                        if elapsed_in_millis.is_positive() && elapsed_in_millis as u64 > (connection_frequency * 3) {
                            match connections.get(&remote_address) {
                                // Disconnect from the peer if the ping request fails to send.
                                Some(channel) => {
                                    if let Err(_) = pings.send_ping(channel).await {
                                        warn!("Ping message failed to send to {}", remote_address);
                                        // Acquire the peer book write lock.
                                        let mut peer_book = context.peer_book.write().await;
                                        peer_book.disconnected_peer(&remote_address);
                                        drop(peer_book);
                                    }
                                }
                                // Disconnect from the peer if the channel is not active.
                                None => {
                                    // Acquire the peer book write lock.
                                    let mut peer_book = context.peer_book.write().await;
                                    peer_book.disconnected_peer(&remote_address);
                                    drop(peer_book);
                                }
                            }
                        }
                    }

                    // Purge peers that haven't responded in five frequency loops.
                    let timeout_duration = ChronoDuration::milliseconds((connection_frequency * 5) as i64);
                    for (remote_address, peer_info) in &connected_peers {
                        if Utc::now() - *peer_info.last_seen() > timeout_duration {
                            // Acquire the peer book write lock.
                            let mut peer_book = context.peer_book.write().await;
                            peer_book.disconnected_peer(&remote_address);
                            drop(peer_book);
                        }
                    }

                    // If we have disconnected from our sync node,
                    // then set our sync state to idle and find a new sync node.
                    if let Ok(mut sync_handler) = sync_handler_lock.try_lock() {
                        let peer_book = context.peer_book.read().await;
                        if peer_book.is_disconnected(&sync_handler.sync_node_address) {
                            if let Some(peer) = peer_book
                                .get_connected()
                                .iter()
                                .max_by(|a, b| a.1.last_seen().cmp(&b.1.last_seen()))
                            {
                                sync_handler.sync_state = SyncState::Idle;
                                sync_handler.sync_node_address = peer.0.clone();
                            };
                        }
                        drop(peer_book)
                    }

                    // Save the peer book to the database.
                    {
                        // Acquire the peer book write lock.
                        let peer_book = context.peer_book.read().await;
                        peer_book
                            .store(&storage)
                            .unwrap_or_else(|error| debug!("Failed to store connected peers in database {}", error));
                        drop(peer_book);
                    }

                    // On every other loop, broadcast a version message to all peers for a periodic sync.
                    if interval_ticker % 2 == 1 && num_connected_peers > 0 {
                        debug!("Sending out periodic version message to peers");
                        // Send a `Version` message to every peer the node is connected to.
                        for (remote_address, _) in &connected_peers {
                            match connections.get(&remote_address) {
                                // Send a version message to peers.
                                // If they are behind, they will attempt to sync.
                                Some(channel) => {
                                    let handshakes = context.handshakes.read().await;
                                    if let Some(handshake) = handshakes.get(&remote_address) {
                                        let version = Version::from(
                                            1u64, // TODO (raychu86) Establish a formal node version
                                            storage.get_latest_block_height(),
                                            *remote_address,
                                            local_address,
                                            handshake.nonce,
                                        );
                                        // Disconnect from the peer if the version request fails to send.
                                        if let Err(_) = channel.write(&version).await {
                                            // Acquire the peer book write lock.
                                            let mut peer_book = context.peer_book.write().await;
                                            peer_book.disconnected_peer(&remote_address);
                                            drop(peer_book);
                                        }
                                    }
                                }
                                // Disconnect from the peer if there is no active connection channel
                                None => {
                                    // Acquire the peer book write lock.
                                    let mut peer_book = context.peer_book.write().await;
                                    peer_book.disconnected_peer(&remote_address);
                                    drop(peer_book);
                                }
                            }
                        }
                    }

                    // Update our memory pool after memory_pool_interval frequency loops.
                    if interval_ticker >= context.memory_pool_interval {
                        if let Ok(sync_handler) = sync_handler_lock.try_lock() {
                            // Ask our sync node for more transactions.
                            if local_address != sync_handler.sync_node_address {
                                if let Some(channel) = connections.get(&sync_handler.sync_node_address) {
                                    if let Err(_) = channel.write(&GetMemoryPool).await {
                                        // Acquire the peer book write lock.
                                        let mut peer_book = context.peer_book.write().await;
                                        peer_book.disconnected_peer(&sync_handler.sync_node_address);
                                        drop(peer_book);
                                    }
                                }
                            }
                        }

                        // Update the node's memory pool.
                        let mut memory_pool = match memory_pool_lock.try_lock() {
                            Ok(memory_pool) => memory_pool,
                            _ => continue,
                        };
                        memory_pool.cleanse(&storage).unwrap_or_else(|error| {
                            debug!("Failed to cleanse memory pool transactions in database {}", error)
                        });
                        memory_pool.store(&storage).unwrap_or_else(|error| {
                            debug!("Failed to store memory pool transaction in database {}", error)
                        });

                        interval_ticker = 0;
                    } else {
                        interval_ticker += 1;
                    }
                }
            }
        });
    }
}
