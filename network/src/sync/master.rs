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

use std::{collections::HashMap, net::SocketAddr, time::Duration};

use crate::{NetworkError, Node, Payload, Peer};
use futures::{pin_mut, select, FutureExt};
use rand::prelude::SliceRandom;
use snarkvm_algorithms::crh::double_sha256;
use snarkvm_objects::{BlockHeader, BlockHeaderHash, Storage};
use tokio::{sync::mpsc, time::Instant};

pub enum SyncInbound {
    BlockHashes(SocketAddr, Vec<BlockHeaderHash>),
    Block(SocketAddr, Vec<u8>),
}

pub struct SyncMaster<S: Storage + Send + Sync + 'static> {
    node: Node<S>,
    incoming: mpsc::Receiver<SyncInbound>,
}

struct SyncBlock {
    address: SocketAddr,
    block: Vec<u8>,
}

impl<S: Storage + Send + Sync + 'static> SyncMaster<S> {
    pub fn new(node: Node<S>) -> (Self, mpsc::Sender<SyncInbound>) {
        let (sender, receiver) = mpsc::channel(256);
        let new = Self {
            node,
            incoming: receiver,
        };
        (new, sender)
    }

    async fn find_sync_nodes(&mut self) -> Vec<Peer> {
        let our_block_height = self.node.expect_sync().current_block_height();
        let mut interesting_peers = vec![];
        for node in self.node.peer_book.connected_peers_snapshot().await {
            if !node.judge() && node.quality.block_height > our_block_height + 1 {
                interesting_peers.push(node);
            }
        }
        interesting_peers.sort_by(|x, y| y.quality.block_height.cmp(&x.quality.block_height));

        // trim nodes close to us if any are > 10 blocks ahead
        if let Some(i) = interesting_peers
            .iter()
            .position(|x| x.quality.block_height <= our_block_height + 10)
        {
            interesting_peers.truncate(i + 1);
        }

        interesting_peers
    }

    async fn block_locator_hashes(&mut self) -> Vec<BlockHeaderHash> {
        match self.node.expect_sync().storage().get_block_locator_hashes() {
            Ok(block_locator_hashes) => block_locator_hashes,
            _ => {
                error!("Unable to get block locator hashes from storage");
                vec![]
            }
        }
    }

    async fn send_sync_messages(&mut self) -> usize {
        let sync_nodes = self.find_sync_nodes().await;

        info!("requested block information from {} peers", sync_nodes.len());
        let block_locator_hashes = self.block_locator_hashes().await;
        let mut sent = 0usize;
        let mut future_set = vec![];
        for peer in sync_nodes.iter() {
            if let Some(handle) = self.node.peer_book.get_peer_handle(peer.address) {
                let block_locator_hashes = block_locator_hashes.clone();
                sent += 1;
                future_set.push(async move {
                    handle.send_payload(Payload::GetSync(block_locator_hashes)).await;
                });
            }
        }
        futures::future::join_all(future_set).await;
        sent
    }

    async fn receive_messages<F: FnMut(SyncInbound) -> bool>(&mut self, timeout_sec: u64, mut handler: F) {
        let end = Instant::now() + Duration::from_secs(timeout_sec);
        loop {
            let timeout = tokio::time::sleep_until(end).fuse();
            pin_mut!(timeout);
            select! {
                msg = self.incoming.recv().fuse() => {
                    if msg.is_none() {
                        break;
                    }
                    if handler(msg.unwrap()) {
                        break;
                    }
                },
                _ = timeout => {
                    break;
                }
            }
        }
    }

    async fn receive_sync_hashes(&mut self, max_message_count: usize) -> HashMap<SocketAddr, Vec<BlockHeaderHash>> {
        const TIMEOUT: u64 = 15;
        let mut received_block_hashes = HashMap::new();

        self.receive_messages(TIMEOUT, |msg| {
            match msg {
                SyncInbound::BlockHashes(addr, hashes) => {
                    received_block_hashes.insert(addr, hashes);
                }
                SyncInbound::Block(_, _) => {
                    warn!("received sync block prematurely");
                }
            }
            //todo: fail if peer sends > 1 block hash packet
            received_block_hashes.len() >= max_message_count
        })
        .await;

        info!(
            "received block information from {} peers in {} seconds",
            received_block_hashes.len(),
            TIMEOUT
        );

        received_block_hashes
    }

    async fn receive_sync_blocks(&mut self, block_count: usize) -> Vec<SyncBlock> {
        const TIMEOUT: u64 = 30;
        let mut blocks = vec![];

        self.receive_messages(TIMEOUT, |msg| {
            match msg {
                SyncInbound::BlockHashes(_, _) => {
                    // late, ignored
                }
                SyncInbound::Block(address, block) => {
                    blocks.push(SyncBlock { address, block });
                }
            }
            blocks.len() >= block_count
        })
        .await;

        info!("received {} blocks in {} seconds", blocks.len(), TIMEOUT);

        blocks
    }

    fn order_block_hashes(input: &[(SocketAddr, Vec<BlockHeaderHash>)]) -> Vec<BlockHeaderHash> {
        let mut block_order = vec![];
        let mut block_index = 0;
        loop {
            let mut found_row = false;
            for (_, hashes) in input {
                if let Some(hash) = hashes.get(block_index) {
                    found_row = true;
                    if let Some(last_hash) = block_order.last() {
                        if last_hash == hash {
                            continue;
                        }
                    }
                    block_order.push(hash.clone());
                }
            }
            block_index += 1;
            if !found_row {
                break;
            }
        }
        block_order
    }

    fn block_peer_map(blocks: &[(SocketAddr, Vec<BlockHeaderHash>)]) -> HashMap<BlockHeaderHash, Vec<SocketAddr>> {
        let mut block_peer_map = HashMap::new();
        for (addr, hashes) in blocks {
            for hash in hashes {
                block_peer_map.entry(hash.clone()).or_insert_with(|| vec![]).push(*addr);
            }
        }
        block_peer_map
    }

    async fn request_blocks(
        &mut self,
        blocks: &[BlockHeaderHash],
        block_peer_map: &HashMap<BlockHeaderHash, Vec<SocketAddr>>,
    ) -> usize {
        let mut peer_block_requests: HashMap<SocketAddr, Vec<BlockHeaderHash>> = HashMap::new();
        let mut sent = 0usize;
        for block in blocks {
            let peers = block_peer_map.get(&block);
            if peers.is_none() {
                continue;
            }
            let random_peer = peers.unwrap().choose(&mut rand::thread_rng());
            if random_peer.is_none() {
                continue;
            }
            peer_block_requests
                .entry(random_peer.unwrap().clone())
                .or_insert_with(|| vec![])
                .push(block.clone());
        }

        let mut future_set = vec![];
        for (addr, request) in peer_block_requests {
            if let Some(peer) = self.node.peer_book.get_peer_handle(addr) {
                // break up requests so the peer wont try to send too large packets
                // for hash in request {
                //     let peer = peer.clone();
                //     sent += 1;
                //     future_set.push(async move { peer.send_payload(Payload::GetBlocks(vec![hash])).await; });
                // }
                sent += request.len();
                future_set.push(async move {
                    peer.send_payload(Payload::GetBlocks(request)).await;
                });
            }
        }
        futures::future::join_all(future_set).await;
        sent
    }

    pub async fn run(mut self) -> Result<(), NetworkError> {
        let hash_requests_sent = self.send_sync_messages().await;

        if hash_requests_sent == 0 {
            return Ok(());
        }

        let received_block_hashes = self.receive_sync_hashes(hash_requests_sent).await;

        if received_block_hashes.is_empty() {
            return Ok(());
        }

        let blocks = received_block_hashes.into_iter().collect::<Vec<_>>();

        let block_order = Self::order_block_hashes(&blocks[..]);

        info!("requesting {} blocks for sync", block_order.len());

        let block_peer_map = Self::block_peer_map(&blocks[..]);

        let sent_block_requests = self.request_blocks(&block_order[..], &block_peer_map).await;

        let received_blocks = self.receive_sync_blocks(sent_block_requests).await;

        info!(
            "received {}/{} blocks for sync",
            received_blocks.len(),
            sent_block_requests
        );

        let mut blocks_by_hash = HashMap::new();

        for block in received_blocks {
            let block_header = &block.block[..BlockHeader::size()];
            let hash = BlockHeaderHash(double_sha256(block_header));
            blocks_by_hash.insert(hash, block);
        }

        for (i, hash) in block_order.iter().enumerate() {
            if let Some(block) = blocks_by_hash.remove(hash) {
                self.node
                    .process_received_block(block.address, block.block, false)
                    .await?;
            } else {
                warn!(
                    "did not receive block {}/{} '{}' by deadline for sync",
                    i,
                    block_order.len(),
                    hash
                );
            }
        }

        self.node.finished_syncing_blocks();
        Ok(())
    }
}
