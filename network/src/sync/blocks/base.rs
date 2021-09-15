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

use std::{net::SocketAddr, time::Duration};

use snarkos_metrics::wrapped_mpsc;
use snarkos_storage::Digest;

use crate::{Node, Peer, SyncInbound};
use anyhow::*;

/// Base sync helpers
pub struct SyncBase {
    pub node: Node,
    pub incoming: wrapped_mpsc::Receiver<SyncInbound>,
}

impl SyncBase {
    pub fn new(node: Node) -> (Self, wrapped_mpsc::Sender<SyncInbound>) {
        let (sender, receiver) = wrapped_mpsc::channel(snarkos_metrics::queues::SYNC_ITEMS, 256);
        let new = Self {
            node,
            incoming: receiver,
        };
        (new, sender)
    }

    pub async fn find_sync_nodes(&self) -> Result<Vec<Peer>> {
        let our_block_height = self.node.storage.canon().await?.block_height;
        let mut interesting_peers = vec![];
        for mut node in self.node.peer_book.connected_peers_snapshot().await {
            let judge_bad = node.judge_bad();
            if !judge_bad && node.quality.block_height as usize > our_block_height + 1 {
                interesting_peers.push(node);
            }
        }
        interesting_peers.sort_by(|x, y| y.quality.block_height.cmp(&x.quality.block_height));

        // trim nodes close to us if any are > 10 blocks ahead
        if let Some(i) = interesting_peers
            .iter()
            .position(|x| x.quality.block_height as usize <= our_block_height + 10)
        {
            interesting_peers.truncate(i + 1);
        }

        if !interesting_peers.is_empty() {
            info!("found {} interesting peers for sync", interesting_peers.len());
            trace!("sync interesting peers = {:?}", interesting_peers);
        }

        Ok(interesting_peers)
    }

    pub async fn block_locator_hashes(node: &Node) -> Result<Vec<Digest>> {
        let forks_of_interest = node
            .storage
            .scan_forks(snarkos_consensus::OLDEST_FORK_THRESHOLD as u32)
            .await?;
        trace!("sync found {} forks", forks_of_interest.len());
        let blocks_of_interest: Vec<Digest> = forks_of_interest.into_iter().map(|(_canon, fork)| fork).collect();
        let mut tips_of_blocks_of_interest: Vec<Digest> = Vec::with_capacity(blocks_of_interest.len());
        for block in blocks_of_interest {
            if tips_of_blocks_of_interest.len() > crate::MAX_BLOCK_SYNC_COUNT as usize {
                debug!("reached limit of blocks of interest in sync block locator hashes");
                break;
            }
            let mut fork_path = node.storage.longest_child_path(&block).await?;
            if fork_path.len() < 2 {
                // a minor fork, we probably don't care
                continue;
            }
            tips_of_blocks_of_interest.push(fork_path.pop().unwrap());
        }
        let hashes = match node
            .storage
            .get_block_locator_hashes(tips_of_blocks_of_interest, snarkos_consensus::OLDEST_FORK_THRESHOLD)
            .await
        {
            Ok(block_locator_hashes) => Ok(block_locator_hashes),
            Err(e) => {
                error!("Unable to get block locator hashes from storage: {:?}", e);
                Err(e)
            }
        }?;

        Ok(hashes)
    }

    /// receives an arbitrary amount of inbound sync messages with a given timeout.
    /// if the passed `handler` callback returns `true`, then the loop is terminated early.
    /// if the sync stream closes, the loop is also terminated early.
    pub async fn receive_messages<F: FnMut(SyncInbound) -> bool>(
        &mut self,
        timeout_sec: u64,
        moving_timeout_sec: u64,
        mut handler: F,
    ) {
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(timeout_sec));
            let extra_time = Duration::from_secs(moving_timeout_sec);

            tokio::pin!(timeout);
            tokio::select! {
                biased;

                _ = timeout.as_mut() => {
                    break;
                }
                msg = self.incoming.recv() => {
                    if msg.is_none() {
                        break;
                    }
                    if handler(msg.unwrap()) {
                        break;
                    }
                    let updated_timeout = timeout.deadline() + extra_time;
                    timeout.as_mut().reset(updated_timeout);
                },
            }
        }
    }

    pub async fn cancel_outstanding_syncs(&self, addresses: &[SocketAddr]) {
        let mut future_set = vec![];
        for addr in addresses {
            if let Some(peer) = self.node.peer_book.get_peer_handle(*addr) {
                future_set.push(async move {
                    peer.cancel_sync().await;
                });
            }
        }
        futures::future::join_all(future_set).await;
    }
}
