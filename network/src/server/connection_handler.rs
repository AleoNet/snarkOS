use crate::{
    message::types::{GetMemoryPool, GetPeers},
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
    /// 3. Purge peers that have not responded in connection_frequency x 2 seconds.
    /// 4. Reselect a sync node if we purged it.
    /// 5. Update our memory pool every connection_frequency x memory_pool_interval seconds.
    /// All errors encountered by the connection handler will be logged to the console but will not stop the thread.
    pub(in crate::server) async fn connection_handler(&self) {
        let context = self.context.clone();
        let memory_pool_lock = self.memory_pool_lock.clone();
        let sync_handler_lock = self.sync_handler_lock.clone();
        let storage = self.storage.clone();
        let connection_frequency = self.connection_frequency;

        // Start a separate thread for the handler.
        task::spawn(async move {
            let mut interval_ticker: u8 = 0;

            loop {
                // There are a lot of potentially expensive and blocking operations here.
                // Let's wait for connection_frequency seconds before starting each loop.
                delay_for(Duration::from_millis(connection_frequency)).await;

                let peer_book = &mut context.peer_book.write().await;
                let connections = context.connections.read().await;
                let pings = &mut context.pings.write().await;

                // We have less peers than our minimum peer requirement. Look for more peers.
                if peer_book.connected_total() < context.min_peers {
                    // Ask our connected peers.
                    for (address, _last_seen) in peer_book.get_connected() {
                        if let Some(channel) = connections.get(&address) {
                            if let Err(_) = channel.write(&GetPeers).await {
                                peer_book.disconnect_peer(address);
                            }
                        }
                    }

                    // Try and connect to our gossiped peers.
                    for (address, _last_seen) in peer_book.get_gossiped() {
                        if address != context.local_address {
                            if let Err(_) = context
                                .handshakes
                                .write()
                                .await
                                .send_request(1u64, storage.get_latest_block_height(), context.local_address, address)
                                .await
                            {
                                peer_book.disconnect_peer(address);
                            }
                        }
                    }
                }

                // Send a ping protocol request to each of our connected peers to maintain the connection.
                for (address, _last_seen) in peer_book.get_connected() {
                    if address != context.local_address {
                        if let Some(channel) = connections.get(&address) {
                            if let Err(_) = pings.send_ping(channel).await {
                                peer_book.disconnect_peer(address);
                            }
                        }
                    }
                }

                // Purge peers that haven't responded in two frequency loops.
                let response_timeout = ChronoDuration::milliseconds((connection_frequency * 2) as i64);

                for (address, last_seen) in peer_book.get_connected() {
                    if Utc::now() - last_seen.clone() > response_timeout {
                        peer_book.disconnect_peer(address);
                    }
                }

                // If we have disconnected from our sync node, then find a new one.
                let mut sync_handler = sync_handler_lock.lock().await;
                if peer_book.disconnected_contains(&sync_handler.sync_node) {
                    match peer_book.get_connected().iter().max_by(|a, b| a.1.cmp(&b.1)) {
                        Some(peer) => sync_handler.sync_node = peer.0.clone(),
                        None => continue,
                    };
                }

                // Store connected peers in database.
                peer_book
                    .store(&storage)
                    .unwrap_or_else(|error| info!("Failed to store connected peers in database {}", error));

                // Update our memory pool after memory_pool_interval frequency loops.
                if interval_ticker >= context.memory_pool_interval {
                    let mut memory_pool = memory_pool_lock.lock().await;

                    memory_pool.cleanse(&storage).unwrap_or_else(|error| {
                        info!("Failed to cleanse memory pool transactions in database {}", error)
                    });

                    memory_pool
                        .store(&storage)
                        .unwrap_or_else(|error| info!("Failed to store memory pool transaction in database {}", error));

                    // Ask our sync node for more transactions.
                    if context.local_address != sync_handler.sync_node {
                        if let Some(channel) = connections.get(&sync_handler.sync_node) {
                            if let Err(_) = channel.write(&GetMemoryPool).await {
                                peer_book.disconnect_peer(sync_handler.sync_node);
                            }
                        }
                    }

                    interval_ticker = 0;
                } else {
                    interval_ticker += 1;
                }

                drop(sync_handler);
            }
        });
    }
}
