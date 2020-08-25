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

use crate::message::{
    types::{GetBlock, GetSync},
    Channel,
};
use snarkos_errors::network::SendError;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_objects::BlockHeaderHash;
use snarkos_storage::Ledger;

use chrono::{DateTime, Utc};
use std::{net::SocketAddr, sync::Arc, time::Duration};
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
    pub sync_node: SocketAddr,
    pub sync_state: SyncState,
    block_headers: Vec<BlockHeaderHash>,
}

impl SyncHandler {
    /// Construct a new `SyncHandler`.
    pub fn new(sync_node: SocketAddr) -> Self {
        Self {
            block_headers: vec![],
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
                if !self.block_headers.contains(&block_hash) {
                    self.block_headers.push(block_hash.clone());
                }
                self.update_syncing(height);
            }
        } else {
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
                info!(
                    "Synced {} Block(s) in {:.2} seconds",
                    storage.get_latest_block_height() - height,
                    (Utc::now() - date_time).num_milliseconds() as f64 / 1000.
                );
                self.update_syncing(storage.get_latest_block_height());
            }

            // Sync up to 10 blocks at once
            for _ in 0..10 {
                if self.block_headers.is_empty() {
                    break;
                }

                channel.write(&GetBlock::new(self.block_headers.remove(0))).await?;
            }

            if self.block_headers.is_empty() {
                delay_for(Duration::from_millis(100)).await;
                if let Ok(block_locator_hashes) = storage.get_block_locator_hashes() {
                    channel.write(&GetSync::new(block_locator_hashes)).await?;
                }
            }
        }

        Ok(())
    }
}
