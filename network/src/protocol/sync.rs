use crate::message::{
    types::{GetBlock, GetSync},
    Channel,
};
use snarkos_errors::network::SendError;
use snarkos_objects::BlockHeaderHash;
use snarkos_storage::BlockStorage;

use chrono::{DateTime, Utc};
use std::{net::SocketAddr, sync::Arc};

pub enum SyncState {
    Idle,
    // (timestamp, block_height)
    Syncing(DateTime<Utc>, u32),
}

/// Manages syncing chain state with a sync node
/// 1. The server_node sends a GetSync message to a sync_node
/// 2. The sync_node responds with a vector of block_headers the server_node is missing
/// 3. The server_node sends a GetBlock message for each BlockHeaderHash in the vector
pub struct SyncHandler {
    pub block_headers: Vec<BlockHeaderHash>,
    pub sync_node: SocketAddr,
    pub sync_state: SyncState,
}

impl SyncHandler {
    pub fn new(bootnode: SocketAddr) -> Self {
        Self {
            block_headers: vec![],
            sync_node: bootnode,
            sync_state: SyncState::Idle,
        }
    }

    pub fn update_syncing(&mut self, block_height: u32) {
        match self.sync_state {
            SyncState::Idle => self.sync_state = SyncState::Syncing(Utc::now(), block_height),
            SyncState::Syncing(date_time, _old_height) => self.sync_state = SyncState::Syncing(date_time, block_height),
        }
    }

    pub async fn increment(&mut self, channel: Arc<Channel>, storage: Arc<BlockStorage>) -> Result<(), SendError> {
        if let SyncState::Syncing(date_time, height) = self.sync_state {
            if self.block_headers.is_empty() {
                info!(
                    "Synced {} Blocks in {:.2} seconds",
                    storage.get_latest_block_height() - height,
                    (Utc::now() - date_time).num_milliseconds() as f64 / 1000.
                );

                self.sync_state = SyncState::Idle;

                if let Ok(block_locator_hashes) = storage.get_block_locator_hashes() {
                    channel.write(&GetSync::new(block_locator_hashes)).await?;
                }
            } else {
                channel.write(&GetBlock::new(self.block_headers.remove(0))).await?;
            }
        }

        Ok(())
    }
}
