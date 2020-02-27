use crate::{
    message::types::{GetMemoryPool, GetPeers},
    Server,
};

use chrono::{Duration as ChronoDuration, Utc};
use std::time::Duration;
use tokio::{task, time::delay_for};

impl Server {
    /// Manage number of active connections according to the connection frequency
    pub(in crate::server) async fn connection_handler(&self) {
        let context = self.context.clone();
        let memory_pool_lock = self.memory_pool_lock.clone();
        let sync_handler_lock = self.sync_handler_lock.clone();
        let storage = self.storage.clone();
        let connection_frequency = self.connection_frequency;

        task::spawn(async move {
            let mut interval_ticker: u8 = 0;

            loop {
                delay_for(Duration::from_millis(connection_frequency)).await;

                let peer_book = &mut context.peer_book.write().await;

                // We have less peers than our minimum peer requirement. Look for more peers
                if peer_book.peers.addresses.len() < context.min_peers as usize {
                    for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if let Err(_) = channel.write(&GetPeers).await {
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }

                    for (socket_addr, _last_seen) in peer_book.gossiped.addresses.clone() {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if socket_addr != context.local_addr {
                                if let Err(_) = context
                                    .handshakes
                                    .write()
                                    .await
                                    .send_request(channel, 1u64, storage.get_latest_block_height(), context.local_addr)
                                    .await
                                {
                                    peer_book.disconnect_peer(&socket_addr);
                                }
                            }
                        }
                    }
                }

                // Maintain a connection with existing peers and update last seen
                for (socket_addr, _last_seen) in peer_book.peers.addresses.clone() {
                    // Ping peers and update last seen if there is a response
                    if socket_addr != context.local_addr {
                        if let Some(channel) = context.connections.read().await.get(&socket_addr) {
                            if let Err(_) = context.pings.write().await.send_ping(channel).await {
                                peer_book.disconnect_peer(&socket_addr);
                            }
                        }
                    }
                }

                // Purge peers that haven't responded in 2 loops
                let response_timeout = ChronoDuration::milliseconds((connection_frequency * 2) as i64);

                for (socket_addr, last_seen) in peer_book.peers.addresses.clone() {
                    if Utc::now() - last_seen.clone() > response_timeout {
                        peer_book.disconnect_peer(&socket_addr);
                    }
                }

                let mut sync_handler = sync_handler_lock.lock().await;
                if peer_book.disconnected_contains(&sync_handler.sync_node) {
                    match peer_book.peers.addresses.iter().max_by(|a, b| a.1.cmp(&b.1)) {
                        Some(peer) => sync_handler.sync_node = peer.0.clone(),
                        None => continue,
                    };
                }

                if interval_ticker >= context.memory_pool_interval {
                    // Also request memory pool and cleanse necessary values
                    let mut memory_pool = memory_pool_lock.lock().await;

                    match (memory_pool.cleanse(&storage), memory_pool.store(&storage)) {
                        (_, _) => {}
                    };

                    if context.local_addr != sync_handler.sync_node {
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
