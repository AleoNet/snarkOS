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
    external::{
        message_types::{GetBlock, GetSync},
        Channel,
        GetMemoryPool,
    },
    peer_manager::PeerManager,
    Environment,
    NetworkError,
};
use snarkos_errors::network::SendError;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};
use snarkos_objects::BlockHeaderHash;
use snarkos_storage::Ledger;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::sleep;

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

/// A stateful component for managing ledger syncing for this node.
pub struct SyncManager {
    environment: Environment,

    /// The address of the sync node
    pub sync_node_address: SocketAddr,
    /// Current state of the sync handler
    pub sync_state: SyncState,
    /// Block headers of blocks that need to be downloaded
    pub block_headers: Vec<BlockHeaderHash>,
    /// Pending blocks - Blocks that have been requested and the time of the request
    pub pending_blocks: HashMap<BlockHeaderHash, DateTime<Utc>>,
}

impl SyncManager {
    ///
    /// Creates a new instance of `SyncHandler`.
    ///
    pub fn new(environment: Environment, sync_node_address: SocketAddr) -> Self {
        Self {
            environment,

            block_headers: vec![],
            pending_blocks: HashMap::new(),
            sync_node_address,
            sync_state: SyncState::Idle,
        }
    }

    ///
    /// Returns `true` if the manager is currently syncing blocks.
    ///
    pub fn is_syncing(&self) -> bool {
        match self.sync_state {
            SyncState::Idle => false,
            SyncState::Syncing(_, _) => true,
        }
    }

    /// Returns if the time of the block request, or None if the block was not requested.
    pub fn is_pending(&self, block_header_hash: &BlockHeaderHash) -> Option<DateTime<Utc>> {
        self.pending_blocks.get(block_header_hash).copied()
    }

    /// Remove the blocks that are now included in the chain.
    pub async fn clear_pending(&mut self) {
        for block_hash in &self.pending_blocks.clone().keys() {
            if !self.environment.storage_read().await.block_hash_exists(block_hash) {
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
        if !hashes.is_empty() {
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
    pub async fn increment(&mut self, channel: Arc<Channel>) -> Result<(), SendError> {
        if let SyncState::Syncing(date_time, height) = self.sync_state {
            let current_block_height = self.environment.current_block_height().await;
            if current_block_height > height {
                debug!(
                    "Synced {} Block(s) in {:.2} seconds",
                    current_block_height - height,
                    (Utc::now() - date_time).num_milliseconds() as f64 / 1000.
                );
                self.update_syncing(current_block_height);
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
                        Utc::now() - *request_time > ChronoDuration::seconds(5)
                    }
                    None => !self
                        .environment
                        .storage_read()
                        .await
                        .block_hash_exists(&block_header_hash),
                };

                if should_request {
                    channel.write(&GetBlock::new(block_header_hash.clone())).await?;
                    self.pending_blocks.insert(block_header_hash, Utc::now());
                }
            }

            // Request more block headers

            if self.pending_blocks.is_empty() {
                sleep(Duration::from_millis(500)).await;

                if let Ok(block_locator_hashes) = self.environment.storage_read().await.get_block_locator_hashes() {
                    channel.write(&GetSync::new(block_locator_hashes)).await?;
                }
            } else {
                for (block_header_hash, request_time) in &self.pending_blocks.clone() {
                    if Utc::now() - *request_time > ChronoDuration::seconds(5) {
                        channel.write(&GetBlock::new(block_header_hash.clone())).await?;
                        self.pending_blocks.insert(block_header_hash.clone(), Utc::now());
                    }
                }
            }

            self.clear_pending().await;
        } else {
            self.clear_pending().await;
        }

        Ok(())
    }

    // TODO (howardwu): Untangle this and find its components new homes.
    /// Manages the number of active connections according to the connection frequency.
    /// 1. Get more connected peers if we are under the minimum number specified by the network context.
    ///     1.1 Ask our connected peers for their peers.
    ///     1.2 Ask our disconnected peers to handshake and become connected.
    /// 2. Maintain connected peers by sending ping messages.
    /// 3. Purge peers that have not responded in sync_interval x 5 seconds.
    /// 4. Reselect a sync node if we purged it.
    /// 5. Update our memory pool every sync_interval x memory_pool_interval seconds.
    /// All errors encountered by the connection handler will be logged to the console but will not stop the thread.
    pub async fn connection_handler(&mut self, peer_manager: PeerManager) -> Result<(), NetworkError> {
        let environment = self.environment.clone();
        let mut interval_ticker: u8 = 0;

        // TODO (howardwu): Implement this.
        {
            // loop {
            //     // Wait for sync_interval seconds in between each loop
            //     sleep(Duration::from_millis(self.environment.sync_interval())).await;
            //
            //     // TODO (howardwu): Rewrite this into a dedicated manager for syncing.
            //     {
            //         // If we have disconnected from our sync node,
            //         // then set our sync state to idle and find a new sync node.
            //         let peer_book = environment.peer_manager_read().await;
            //         if peer_book.is_disconnected(&self.sync_node_address).await {
            //             if let Some(peer) = peer_book
            //                 .connected_peers()
            //                 .await
            //                 .iter()
            //                 .max_by(|a, b| a.1.last_seen().cmp(&b.1.last_seen()))
            //             {
            //                 self.sync_state = SyncState::Idle;
            //                 self.sync_node_address = peer.0.clone();
            //             };
            //         }
            //         drop(peer_book);
            //
            //         // Update our memory pool after memory_pool_interval frequency loops.
            //         if interval_ticker >= environment.memory_pool_interval() {
            //             // Ask our sync node for more transactions.
            //             if *environment.local_address() != self.sync_node_address {
            //                 if let Some(channel) = peer_manager.get_channel(&self.sync_node_address) {
            //                     if let Err(_) = channel.write(&GetMemoryPool).await {
            //                         // Acquire the peer book write lock.
            //                         let mut peer_book = environment.peer_manager_write().await;
            //                         peer_book.disconnect_from_peer(&self.sync_node_address).await?;
            //                         drop(peer_book);
            //                     }
            //                 }
            //             }
            //
            //             // Update this node's memory pool.
            //             let mut memory_pool = match self.environment.memory_pool().try_lock() {
            //                 Ok(memory_pool) => memory_pool,
            //                 _ => continue,
            //             };
            //             memory_pool
            //                 .cleanse(&*self.environment.storage_read().await)
            //                 .unwrap_or_else(|error| {
            //                     debug!("Failed to cleanse memory pool transactions in database {}", error)
            //                 });
            //             memory_pool
            //                 .store(&*self.environment.storage_read().await)
            //                 .unwrap_or_else(|error| {
            //                     debug!("Failed to store memory pool transaction in database {}", error)
            //                 });
            //
            //             interval_ticker = 0;
            //         } else {
            //             interval_ticker += 1;
            //         }
            //     }
            // }
        }

        Ok(())
    }
}
