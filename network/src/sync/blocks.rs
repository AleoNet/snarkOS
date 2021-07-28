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

use std::net::SocketAddr;

use snarkvm_dpc::{Block, BlockHeaderHash, Storage};

use snarkos_consensus::error::ConsensusError;
use snarkos_metrics::{self as metrics, misc::*};

use crate::{master::SyncInbound, message::*, NetworkError, Node};

impl<S: Storage + Send + std::marker::Sync + 'static> Node<S> {
    ///
    /// Sends a `GetSync` request to the given sync node.
    ///
    pub async fn update_blocks(&self, sync_node: SocketAddr) {
        let block_locator_hashes = match self.expect_sync().storage().get_block_locator_hashes() {
            Ok(block_locator_hashes) => block_locator_hashes,
            _ => {
                error!("Unable to get block locator hashes from storage");
                return;
            }
        };

        info!("Updating blocks from {}", sync_node);

        // Send a GetSync to the selected sync node.
        self.peer_book
            .send_to(sync_node, Payload::GetSync(block_locator_hashes))
            .await;
    }

    /// Broadcast block to connected peers
    pub async fn propagate_block(&self, block_bytes: Vec<u8>, block_miner: SocketAddr) {
        debug!("Propagating a block to peers");

        for remote_address in self.connected_peers() {
            if remote_address != block_miner {
                // Send a `Block` message to the connected peer.
                self.peer_book
                    .send_to(remote_address, Payload::Block(block_bytes.clone()))
                    .await;
            }
        }
    }

    /// A peer has sent us a new block to process.
    pub(crate) async fn received_block(
        &self,
        remote_address: SocketAddr,
        block: Vec<u8>,
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
            self.process_received_block(remote_address, block, is_block_new).await?;
        } else {
            let sender = self.master_dispatch.read().await;
            if let Some(sender) = &*sender {
                sender.send(SyncInbound::Block(remote_address, block)).await.ok();
            }
        }
        Ok(())
    }

    pub(super) async fn process_received_block(
        &self,
        remote_address: SocketAddr,
        block: Vec<u8>,
        is_block_new: bool,
    ) -> Result<(), NetworkError> {
        let block_struct = match Block::deserialize(&block) {
            Ok(block) => block,
            Err(error) => {
                error!(
                    "Failed to deserialize received block from {}: {}",
                    remote_address, error
                );
                return Err(error.into());
            }
        };

        info!(
            "Received block from {} of epoch {} with hash {} (current head {})",
            remote_address,
            block_struct.header.time,
            block_struct.header.get_hash(),
            self.expect_sync().current_block_height(),
        );

        // Verify the block and insert it into the storage.
        let block_validity = self.expect_sync().consensus.receive_block(&block_struct, false).await;

        if let Err(ConsensusError::PreExistingBlock) = block_validity {
            if is_block_new {
                metrics::increment_counter!(DUPLICATE_BLOCKS);
            } else {
                metrics::increment_counter!(DUPLICATE_SYNC_BLOCKS);
            }
        }

        // This is a non-sync Block, send it to our peers.
        if block_validity.is_ok() && is_block_new {
            self.propagate_block(block, remote_address).await;
        }

        Ok(())
    }

    /// A peer has requested a block.
    pub(crate) async fn received_get_blocks(
        &self,
        remote_address: SocketAddr,
        header_hashes: Vec<BlockHeaderHash>,
    ) -> Result<(), NetworkError> {
        for hash in header_hashes.into_iter().take(crate::MAX_BLOCK_SYNC_COUNT as usize) {
            let block = self.expect_sync().storage().get_block(&hash)?;

            // Send a `SyncBlock` message to the connected peer.
            self.peer_book
                .send_to(remote_address, Payload::SyncBlock(block.serialize()?))
                .await;
        }

        Ok(())
    }

    /// A peer has requested our chain state to sync with.
    pub(crate) async fn received_get_sync(
        &self,
        remote_address: SocketAddr,
        block_locator_hashes: Vec<BlockHeaderHash>,
    ) -> Result<(), NetworkError> {
        let sync = {
            let storage = self.expect_sync().storage();

            let latest_shared_hash = storage.get_latest_shared_hash(block_locator_hashes)?;
            let shared_height = storage.get_block_number(&latest_shared_hash)?;
            let current_height = storage.get_current_block_height();

            let mut max_sync_height = current_height;

            // if the requester is behind more than MAX_BLOCK_SYNC_COUNT blocks
            if current_height > shared_height + crate::MAX_BLOCK_SYNC_COUNT {
                // send no more than MAX_BLOCK_SYNC_COUNT
                max_sync_height = shared_height + crate::MAX_BLOCK_SYNC_COUNT;
            }

            let mut block_hashes = Vec::with_capacity((max_sync_height - shared_height) as usize);

            for block_num in shared_height + 1..=max_sync_height {
                block_hashes.push(storage.get_block_hash(block_num)?);
            }

            // send block hashes to requester
            block_hashes
        };

        // send a `Sync` message to the connected peer.
        self.peer_book.send_to(remote_address, Payload::Sync(sync)).await;

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
