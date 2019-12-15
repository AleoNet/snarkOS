use crate::base::{send_block_request, send_sync_request};
use snarkos_objects::BlockHeaderHash;
use snarkos_storage::BlockStorage;

use chrono::{DateTime, Utc};
use snarkos_errors::network::SendError;
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

pub enum SyncState {
    Idle,
    // Store timestamp and block height
    Syncing(DateTime<Utc>, u32),
}

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
}

pub async fn increment_sync_handler(
    sync_handler_lock: Arc<Mutex<SyncHandler>>,
    storage: Arc<BlockStorage>,
) -> Result<(), SendError> {
    let mut sync_handler = sync_handler_lock.lock().await;

    if let SyncState::Syncing(date_time, height) = sync_handler.sync_state {
        if sync_handler.block_headers.is_empty() {
            let elapsed_time_seconds = (Utc::now() - date_time).num_milliseconds() as f64 / 1000.;
            let num_blocks_downloaded = storage.get_latest_block_height() - height;
            info!(
                "Synced {} Blocks in {:.2} seconds",
                num_blocks_downloaded, elapsed_time_seconds
            );

            sync_handler.sync_state = SyncState::Idle;

            if let Ok(block_locator_hashes) = storage.get_block_locator_hashes() {
                send_sync_request(sync_handler.sync_node, block_locator_hashes).await?;
            }
        } else {
            let block_hash = sync_handler.block_headers.remove(0);
            send_block_request(sync_handler.sync_node, block_hash).await?;
        }
    }

    Ok(())
}
