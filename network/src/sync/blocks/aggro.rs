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

use std::sync::Arc;

use crate::{Cache, Node, Payload, Peer, SyncBase, SyncInbound};
use anyhow::*;
use snarkos_metrics::wrapped_mpsc;
use snarkos_storage::Digest;
use tokio::sync::RwLock;

/// Aggressive, continuous sync process that pulls peers entire canon trees.
pub struct SyncAggro {
    base: SyncBase,
}

impl SyncAggro {
    pub fn new(node: Node) -> (Self, wrapped_mpsc::Sender<SyncInbound>) {
        let (base, sender) = SyncBase::new(node);
        let new = Self { base };
        (new, sender)
    }

    async fn send_sync_messages(&mut self, nodes: Vec<(Peer, Vec<Digest>)>) -> Result<usize> {
        info!("requested block information from {} peers", nodes.len());
        let mut future_set = vec![];

        for (peer, hashes) in nodes {
            if let Some(handle) = self.base.node.peer_book.get_peer_handle(peer.address) {
                future_set.push(async move {
                    handle.send_payload(Payload::GetSync(hashes), None).await;
                });
            }
        }
        let sent = future_set.len();
        futures::future::join_all(future_set).await;
        Ok(sent)
    }

    pub async fn run(mut self) -> Result<()> {
        let sync_nodes = self.base.find_sync_nodes().await?;

        if sync_nodes.is_empty() {
            return Ok(());
        }

        self.base.node.register_block_sync_attempt();

        let block_locator_hashes = SyncBase::block_locator_hashes(&self.base.node).await?;

        let peer_syncs = {
            sync_nodes
                .iter()
                .map(|peer| (peer.clone(), block_locator_hashes.clone()))
                .collect()
        };

        let hash_requests_sent = self.send_sync_messages(peer_syncs).await?;

        if hash_requests_sent == 0 {
            return Ok(());
        }

        let received_hashes = Arc::new(RwLock::new(Cache::<1024, 32>::default()));

        let node = self.base.node.clone();
        self.base
            .receive_messages(60, 3, |msg| {
                match msg {
                    SyncInbound::BlockHashes(peer, hashes) => {
                        debug!("received {} sync hashes from {}", hashes.len(), peer);
                        let hashes: Vec<Digest> = hashes.into_iter().map(|x| x.0.into()).collect::<Vec<_>>();
                        if hashes.is_empty() {
                            return false;
                        }
                        let last_hash = hashes.last().unwrap().clone();

                        let node = node.clone();
                        let received_hashes = received_hashes.clone();
                        tokio::spawn(async move {
                            let mut hashes_trimmed = Vec::with_capacity(hashes.len());
                            {
                                let received_hashes = received_hashes.read().await;
                                for hash in hashes {
                                    if !received_hashes.contains(&hash[..]) {
                                        hashes_trimmed.push(hash);
                                    }
                                }
                            }

                            let early_block_states = match node.storage.get_block_states(&hashes_trimmed[..]).await {
                                Ok(x) => x,
                                Err(e) => {
                                    warn!("failed to get block states: {:?}", e);
                                    return;
                                }
                            };

                            let blocks: Vec<_> = hashes_trimmed
                                .into_iter()
                                .zip(early_block_states.iter())
                                .filter(|(_, status)| matches!(status, snarkos_storage::BlockStatus::Unknown))
                                .map(|(hash, _)| hash)
                                .collect();
                            debug!("requesting {} sync blocks from {}", blocks.len(), peer);

                            if let Some(peer) = node.peer_book.get_peer_handle(peer) {
                                peer.expecting_sync_blocks(blocks.len() as u32).await;
                                peer.send_payload(Payload::GetSync(vec![last_hash]), None).await;
                                if blocks.is_empty() {
                                    return;
                                }
                                peer.send_payload(Payload::GetBlocks(blocks), None).await;
                            }
                        });
                    }
                    SyncInbound::Block(peer, block, peer_height) => {
                        let node = node.clone();
                        let received_hashes = received_hashes.clone();
                        tokio::spawn(async move {
                            received_hashes.write().await.push(&block[..]);
                            match node.process_received_block(peer, block, peer_height, false).await {
                                Err(e) => warn!("failed to process received block from {}: {:?}", peer, e),
                                Ok(()) => (),
                            }
                        });
                    }
                }
                false
            })
            .await;

        let sync_addresses = sync_nodes.iter().map(|x| x.address).collect::<Vec<_>>();

        self.base.cancel_outstanding_syncs(&sync_addresses[..]).await;

        Ok(())
    }
}
