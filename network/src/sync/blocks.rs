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

use snarkos_storage::{BlockStatus, Digest, VMBlock};
use snarkvm_dpc::{
    testnet1::{instantiated::Components, Transaction},
    Block,
    BlockHeaderHash,
};

use snarkos_consensus::error::ConsensusError;

use crate::{master::SyncInbound, message::*, NetworkError, Node};
use anyhow::*;

impl Node {
    /// Broadcast block to connected peers
    pub fn propagate_block(&self, block_bytes: Vec<u8>, height: Option<u32>, block_miner: SocketAddr) {
        debug!("Propagating a block to peers");

        let connected_peers = self.connected_peers();
        let peer_book = self.peer_book.clone();
        tokio::spawn(async move {
            let mut futures = Vec::with_capacity(connected_peers.len());
            for remote_address in connected_peers.iter() {
                if remote_address != &block_miner {
                    futures.push(peer_book.send_to(*remote_address, Payload::Block(block_bytes.clone(), height)));
                }
            }
            tokio::time::timeout(Duration::from_secs(1), futures::future::join_all(futures))
                .await
                .ok();
        });
    }

    /// A peer has sent us a new block to process.
    pub(crate) async fn received_block(
        &self,
        remote_address: SocketAddr,
        block: Vec<u8>,
        height: Option<u32>,
        is_block_new: bool,
    ) -> Result<(), NetworkError> {
        let block_size = block.len();
        let max_block_size = self.expect_sync().max_block_size();

        if block_size > max_block_size {
            error!(
                "Received block from {} that is too big ({} > {})",
                remote_address, block_size, max_block_size
            );
            return Err(NetworkError::ConsensusError(ConsensusError::BlockTooLarge(
                block_size,
                max_block_size,
            )));
        }

        if is_block_new {
            let node_clone = self.clone();
            tokio::spawn(async move {
                if let Err(e) = node_clone
                    .process_received_block(remote_address, block, height, is_block_new)
                    .await
                {
                    warn!("error accepting received block: {:?}", e);
                }
            });
        } else {
            let sender = self.master_dispatch.read().await;
            if let Some(sender) = &*sender {
                sender
                    .send(SyncInbound::Block(remote_address, block, height))
                    .await
                    .ok();
            }
        }
        Ok(())
    }

    pub(super) async fn process_received_block(
        &self,
        remote_address: SocketAddr,
        block: Vec<u8>,
        height: Option<u32>,
        is_block_new: bool,
    ) -> Result<(), NetworkError> {
        let block_struct = match Block::<Transaction<Components>>::deserialize(&block) {
            Ok(block) => block,
            Err(error) => {
                error!(
                    "Failed to deserialize received block from {}: {}",
                    remote_address, error
                );
                return Err(error.into());
            }
        };
        let block_struct = <Block<Transaction<Components>> as VMBlock>::serialize(&block_struct)?;

        let canon = self.storage.canon().await?;

        info!(
            "Received block from {} of epoch {}{} with hash {} (current head {})",
            remote_address,
            block_struct.header.time,
            if let Some(h) = height {
                format!(" (peer's height {})", h)
            } else {
                "".to_string()
            },
            block_struct.header.hash(),
            canon.block_height,
        );

        // Verify the block and insert it into the storage.
        let block_validity = self.expect_sync().consensus.receive_block(block_struct).await;

        if block_validity && is_block_new {
            // This is a non-sync Block, send it to our peers.
            self.propagate_block(block, height, remote_address);
        }

        Ok(())
    }

    /// A peer has requested a block.
    pub(crate) async fn received_get_blocks(
        &self,
        remote_address: SocketAddr,
        header_hashes: Vec<Digest>,
    ) -> Result<(), NetworkError> {
        for hash in header_hashes.into_iter().take(crate::MAX_BLOCK_SYNC_COUNT as usize) {
            let block = self.storage.get_block(&hash).await?;
            let height = match self.storage.get_block_state(&block.header.hash()).await? {
                BlockStatus::Committed(h) => Some(h as u32),
                _ => None,
            };

            // Send a `SyncBlock` message to the connected peer.
            self.peer_book
                .send_to(remote_address, Payload::SyncBlock(block.serialize(), height))
                .await;
        }

        Ok(())
    }

    /// A peer has requested our chain state to sync with.
    pub(crate) async fn received_get_sync(
        &self,
        remote_address: SocketAddr,
        block_locator_hashes: Vec<Digest>,
    ) -> Result<(), NetworkError> {
        let sync_hashes = self
            .storage
            .find_sync_blocks(&block_locator_hashes[..], crate::MAX_BLOCK_SYNC_COUNT as usize)
            .await?
            .into_iter()
            .map(|x| x.bytes::<32>().map(BlockHeaderHash))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| anyhow!("invalid block header size in locator hash"))?;

        // send a `Sync` message to the connected peer.
        self.peer_book.send_to(remote_address, Payload::Sync(sync_hashes)).await;

        Ok(())
    }

    /// A peer has sent us their chain state.
    pub(crate) async fn received_sync(&self, remote_address: SocketAddr, block_hashes: Vec<BlockHeaderHash>) {
        let sender = self.master_dispatch.read().await;
        if let Some(sender) = &*sender {
            sender
                .send(SyncInbound::BlockHashes(remote_address, block_hashes))
                .await
                .ok();
        }
    }
}
