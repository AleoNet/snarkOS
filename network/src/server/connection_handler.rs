use crate::{
    message::types::{GetMemoryPool, GetPeers},
    Server,
};

use chrono::{Duration as ChronoDuration, Utc};
use std::time::Duration;
use tokio::{task, time::delay_for};

impl Server {
    /// Manages the number of active connections according to the connection frequency.
    ///
    /// 1. Get more connected peers if we are under the minimum number specified by the network context.
    ///     1.1 Ask our connected peers for their peers.
    ///     1.2 Ask our gossiped peers to handshake and become connected.
    /// 2. Maintain connected peers by sending ping messages.
    /// 3. Purge peers that have not responded in connection_frequency x 2 seconds.
    /// 4. Reselect a sync node if we purged it.
    /// 5. Update our memory pool every connection_frequency x memory_pool_interval seconds.
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

                // We have less peers than our minimum peer requirement. Look for more peers.
                if peer_book.peers.addresses.len() < context.min_peers as usize {
                    // Ask our connected peers.
                    for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if let Err(_) = channel.write(&GetPeers).await {
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }

                    // Try and connect to our gossiped peers.
                    for (socket_addr, _last_seen) in peer_book.gossiped.addresses.clone() {
                        if socket_addr != context.local_address {
                            if let Err(_) = context
                                .handshakes
                                .write()
                                .await
                                .send_request(
                                    1u64,
                                    storage.get_latest_block_height(),
                                    context.local_address,
                                    socket_addr,
                                )
                                .await
                            {
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }
                }

                // Send a ping protocol request to each of our connected peers to maintain the connection.
                for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                    if socket_addr != context.local_address {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if let Err(_) = context.pings.write().await.send_ping(channel).await {
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }
                }

                // Purge peers that haven't responded in two frequency loops.
                let response_timeout = ChronoDuration::milliseconds((connection_frequency * 2) as i64);

                for (socket_addr, last_seen) in peer_book.peers.addresses.clone() {
                    if Utc::now() - last_seen.clone() > response_timeout {
                        peer_book.disconnect_peer(&socket_addr);
                    }
                }

                // If we have disconnected from our sync node, then find a new one.
                let mut sync_handler = sync_handler_lock.lock().await;
                if peer_book.disconnected_contains(&sync_handler.sync_node) {
                    match peer_book.peers.addresses.iter().max_by(|a, b| a.1.cmp(&b.1)) {
                        Some(peer) => sync_handler.sync_node = peer.0.clone(),
                        None => continue,
                    };
                }

                // Update our memory pool after memory_pool_interval frequency loops.
                if interval_ticker >= context.memory_pool_interval {
                    let mut memory_pool = memory_pool_lock.lock().await;

                    match (memory_pool.cleanse(&storage), memory_pool.store(&storage)) {
                        (_, _) => {}
                    };

                    // Ask our sync node for more transactions.
                    if context.local_address != sync_handler.sync_node {
                        if let Some(channel) = context.connections.read().await.get(&sync_handler.sync_node) {
                            if let Err(_) = channel.write(&GetMemoryPool).await {
                                peer_book.disconnect_peer(&sync_handler.sync_node);
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
