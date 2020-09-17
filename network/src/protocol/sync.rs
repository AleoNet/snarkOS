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
    message_types::{GetBlock, GetSync},
    outbound::Channel,
};
use snarkos_errors::network::SendError;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_objects::BlockHeaderHash;
use snarkos_storage::Ledger;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::delay_for;

#[derive(Clone, PartialEq)]
pub enum SyncState {
    Idle,
    /// (timestamp, block_height)
    Syncing(DateTime<Utc>, u32),
}

/// Manages syncing chain state with a sync node.
/// 1. The server_node sends a GetSync message to a sync_node.
/// 2. The sync_node responds with a Sync message with block_headers the server_node is missing.
/// 3. The server_node sends a GetBlock message for each BlockHeaderHash in the message.
pub struct SyncHandler {
    /// The address of the sync node
    pub sync_node: SocketAddr,
    /// Current state of the sync handler
    pub sync_state: SyncState,
    /// Block headers of blocks that need to be downloaded
    pub block_headers: Vec<BlockHeaderHash>,
    /// Pending blocks - Blocks that have been requested and the time of the request
    pub pending_blocks: HashMap<BlockHeaderHash, DateTime<Utc>>,
}

impl SyncHandler {
    /// Construct a new `SyncHandler`.
    pub fn new(sync_node: SocketAddr) -> Self {
        Self {
            block_headers: vec![],
            pending_blocks: HashMap::new(),
            sync_node,
            sync_state: SyncState::Idle,
        }
    }

    /// Returns if the sync handler is currently syncing blocks
    pub fn is_syncing(&self) -> bool {
        match self.sync_state {
            SyncState::Idle => false,
            SyncState::Syncing(_, _) => true,
        }
    }

    /// Returns if the time of the block request, or None if the block was not requested.
    pub fn is_pending(&self, block_header_hash: &BlockHeaderHash) -> Option<DateTime<Utc>> {
        self.pending_blocks.get(block_header_hash).map(|time| time.clone())
    }

    /// Remove the blocks that are now included in the chain.
    pub fn clear_pending<T: Transaction, P: LoadableMerkleParameters>(&mut self, storage: Arc<Ledger<T, P>>) {
        for (block_hash, _time_sent) in &self.pending_blocks.clone() {
            if !storage.block_hash_exists(&block_hash) {
                self.pending_blocks.remove(block_hash);
            }
        }
    }

    /// Set the SyncState to syncing and update the latest block height.
    pub fn update_syncing(&mut self, block_height: u32) {
        match self.sync_state {
            SyncState::Idle => {
                info!("Syncing blocks");
                self.sync_state = SyncState::Syncing(Utc::now(), block_height)
            }
            SyncState::Syncing(_date_time, _old_height) => {
                self.sync_state = SyncState::Syncing(Utc::now(), block_height)
            }
        }
    }

    /// Process a vector of block header hashes.
    /// Push new hashes to the sync handler so we can ask the sync node for them.
    pub fn receive_hashes(&mut self, hashes: Vec<BlockHeaderHash>, height: u32) {
        if hashes.len() > 0 {
            for block_hash in hashes {
                if !self.block_headers.contains(&block_hash) && self.pending_blocks.get(&block_hash).is_none() {
                    self.block_headers.push(block_hash.clone());
                }
                self.update_syncing(height);
            }
        } else if self.pending_blocks.is_empty() {
            info!("Sync state is set to Idle");
            self.sync_state = SyncState::Idle;
        }
    }

    /// Finish syncing or ask for the next block from the sync node.
    pub async fn increment<T: Transaction, P: LoadableMerkleParameters>(
        &mut self,
        channel: Arc<Channel>,
        storage: Arc<Ledger<T, P>>,
    ) -> Result<(), SendError> {
        if let SyncState::Syncing(date_time, height) = self.sync_state {
            if storage.get_latest_block_height() > height {
                debug!(
                    "Synced {} Block(s) in {:.2} seconds",
                    storage.get_latest_block_height() - height,
                    (Utc::now() - date_time).num_milliseconds() as f64 / 1000.
                );
                self.update_syncing(storage.get_latest_block_height());
            }

            // Sync up to 3 blocks at once
            for _ in 0..3 {
                if self.block_headers.is_empty() {
                    break;
                }

                let block_header_hash = self.block_headers.remove(0);

                // If block is not pending, then ask the sync node for the block.
                let should_request = match self.pending_blocks.get(&block_header_hash) {
                    Some(request_time) => {
                        // Request the block again if the block was not downloaded in 5 seconds
                        Utc::now() - request_time.clone() > ChronoDuration::seconds(5)
                    }
                    None => !storage.block_hash_exists(&block_header_hash),
                };

                if should_request {
                    channel.write(&GetBlock::new(block_header_hash.clone())).await?;
                    self.pending_blocks.insert(block_header_hash, Utc::now());
                }
            }

            // Request more block headers

            if self.pending_blocks.is_empty() {
                delay_for(Duration::from_millis(500)).await;

                if let Ok(block_locator_hashes) = storage.get_block_locator_hashes() {
                    channel.write(&GetSync::new(block_locator_hashes)).await?;
                }
            } else {
                for (block_header_hash, request_time) in &self.pending_blocks.clone() {
                    if Utc::now() - request_time.clone() > ChronoDuration::seconds(5) {
                        channel.write(&GetBlock::new(block_header_hash.clone())).await?;
                        self.pending_blocks.insert(block_header_hash.clone(), Utc::now());
                    }
                }
            }

            self.clear_pending(Arc::clone(&storage));
        } else {
            self.clear_pending(Arc::clone(&storage));
        }

        Ok(())
    }
}
