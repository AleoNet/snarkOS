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

use snarkos_metrics::{
    self as metrics,
    inbound::{self, *},
    misc,
    outbound,
};
use tokio::task;

use crate::{KnownNetworkMessage, NetworkError, Node, Payload, Peer, State};

use super::network::PeerIOHandle;

use std::time::Instant;

impl Peer {
    pub(super) async fn inner_dispatch_payload(
        &mut self,
        node: &Node,
        network: &mut PeerIOHandle,
        time_received: Option<Instant>,
        payload: Result<Payload, NetworkError>,
    ) -> Result<(), NetworkError> {
        let payload = payload?;
        self.quality.see();
        self.quality.num_messages_received += 1;
        metrics::increment_counter!(inbound::ALL_SUCCESSES);

        let source = self.address;

        // If message is a `SyncBlock` message, log it as a trace.
        match payload {
            Payload::SyncBlock(..) => trace!("Received a '{}' message from {}", payload, source),
            _ => debug!("Received a '{}' message from {}", payload, source),
        }

        match payload {
            Payload::Transaction(transaction) => {
                metrics::increment_counter!(inbound::TRANSACTIONS);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        if let Err(e) = node.received_memory_pool_transaction(source, transaction).await {
                            warn!("Received an invalid transaction from a peer: {}", e);
                            if let Some(peer) = node.peer_book.get_peer_handle(source) {
                                peer.fail().await;
                            }
                        }
                    });
                }
            }
            Payload::Block(block, height) => {
                metrics::increment_counter!(inbound::BLOCKS);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        // Check if the message hasn't already been processed recently if it's a `Block`.
                        // The node should also reject them while syncing, as it is bound to receive them later.
                        if node.inbound_cache.lock().await.contains(&block) {
                            metrics::increment_counter!(misc::DUPLICATE_BLOCKS);
                            return;
                        }

                        if node.state() == State::Syncing {
                            return;
                        }

                        if let Err(e) = node.received_block(source, block, height, true).await {
                            warn!("Received an invalid block from a peer: {}", e);
                            if let Some(peer) = node.peer_book.get_peer_handle(source) {
                                peer.fail().await;
                            }
                        }
                    });
                }
            }
            Payload::SyncBlock(block, height) => {
                metrics::increment_counter!(inbound::SYNCBLOCKS);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        if let Err(e) = node.received_block(source, block, height, false).await {
                            warn!("Received an invalid block from a peer: {}", e);
                            if let Some(peer) = node.peer_book.get_peer_handle(source) {
                                peer.fail().await;
                            }
                            return;
                        };

                        // Update the peer and possibly finish the sync process.
                        if let Some(peer) = node.peer_book.get_peer_handle(source) {
                            peer.got_sync_block().await;
                        }
                    });
                }
            }
            Payload::GetBlocks(hashes) => {
                metrics::increment_counter!(inbound::GETBLOCKS);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        if let Err(e) = node.received_get_blocks(source, hashes, time_received).await {
                            warn!("failed to send sync blocks to peer: {:?}", e);
                        }
                    });
                }
            }
            Payload::GetMemoryPool => {
                metrics::increment_counter!(inbound::GETMEMORYPOOL);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        if let Err(e) = node.received_get_memory_pool(source, time_received).await {
                            warn!("Failed to procure the memory pool for a peer: {:?}", e);
                        }
                    });
                }
            }
            Payload::MemoryPool(mempool) => {
                metrics::increment_counter!(inbound::MEMORYPOOL);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        if let Err(e) = node.received_memory_pool(mempool).await {
                            warn!("Received an invalid memory pool from a peer: {}", e);
                            if let Some(peer) = node.peer_book.get_peer_handle(source) {
                                peer.fail().await;
                            }
                        }
                    });
                }
            }
            Payload::GetSync(getsync) => {
                metrics::increment_counter!(inbound::GETSYNC);

                if node.sync().is_some() {
                    let node = node.clone();
                    task::spawn(async move {
                        if let Err(e) = node.received_get_sync(source, getsync, time_received).await {
                            warn!("Failed to procure sync blocks for a peer: {}", e);
                        }
                    });
                }
            }
            Payload::Sync(sync) => {
                metrics::increment_counter!(inbound::SYNCS);

                if node.sync().is_some() {
                    if sync.is_empty() {
                        // An empty `Sync` is unexpected, as `GetSync` requests are only
                        // sent to peers that declare a greater block height.
                        warn!("{} doesn't have sync blocks to share", source);
                        if let Some(peer) = node.peer_book.get_peer_handle(source) {
                            peer.fail().await;
                        }
                    } else {
                        trace!("Received {} sync block hashes from {}", sync.len(), source);
                        let node = node.clone();
                        task::spawn(async move {
                            node.received_sync(source, sync).await;
                        });
                    }
                }
            }
            Payload::GetPeers => {
                metrics::increment_counter!(inbound::GETPEERS);

                let node = node.clone();
                task::spawn(async move {
                    node.send_peers(source, time_received).await;
                });
            }
            Payload::Peers(peers) => {
                metrics::increment_counter!(inbound::PEERS);

                let node = node.clone();
                task::spawn(async move {
                    node.process_inbound_peers(source, peers).await;
                });
            }
            Payload::Ping(block_height) => {
                network.write_payload(&Payload::Pong).await?;
                debug!("Sent a '{}' message to {}", Payload::Pong, self.address);
                self.quality.block_height = block_height;
                metrics::increment_counter!(PINGS);

                // Pongs are sent without going through the outbound handler,
                // so the outbound metric needs to be incremented here
                metrics::increment_counter!(outbound::ALL_SUCCESSES);

                // Relay the height to the known network.
                if let Some(known_network) = node.known_network() {
                    let _ = known_network
                        .sender
                        .try_send(KnownNetworkMessage::Height(source, block_height));
                }
            }
            Payload::Pong => {
                if self.quality.expecting_pong {
                    let rtt = self
                        .quality
                        .last_ping_sent
                        .map(|x| x.elapsed().as_millis() as u64)
                        .unwrap_or(u64::MAX);
                    trace!("RTT for {} is {}ms", source, rtt);
                    self.quality.expecting_pong = false;
                    self.quality.rtt_ms = rtt;
                } else {
                    self.fail();
                }
                metrics::increment_counter!(PONGS);
            }
            Payload::Unknown => {
                metrics::increment_counter!(inbound::UNKNOWN);
                warn!("Unknown payload received; this could indicate that the client you're using is out-of-date");
            }
        }

        Ok(())
    }

    pub(super) async fn dispatch_payload(
        &mut self,
        node: &Node,
        network: &mut PeerIOHandle,
        time_received: Option<Instant>,
        payload: Result<Payload, NetworkError>,
    ) -> Result<(), NetworkError> {
        match self.inner_dispatch_payload(node, network, time_received, payload).await {
            Ok(()) => (),
            Err(e) => {
                if e.is_trivial() {
                    trace!("Unable to read message from {}: {}", self.address, e);
                } else {
                    warn!("Unable to read message from {}: {}", self.address, e);
                }
                return Err(e);
            }
        }
        Ok(())
    }

    pub(super) fn deserialize_payload(&self, payload: Result<&[u8], NetworkError>) -> Result<Payload, NetworkError> {
        let payload = payload?;
        let payload = Payload::deserialize(payload)?;
        Ok(payload)
    }
}
