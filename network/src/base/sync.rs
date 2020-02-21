use crate::{
    base::Context,
    message::{
        types::{Block, GetBlock, GetSync, Transaction},
        Channel,
    },
};
use snarkos_errors::network::SendError;
use snarkos_objects::{BlockHeaderHash, Transaction as TransactionStruct};
use snarkos_storage::BlockStorage;

use chrono::{DateTime, Utc};
use snarkos_consensus::miner::{Entry, MemoryPool};
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
    channel: Arc<Channel>,
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
                channel.write(&GetSync::new(block_locator_hashes)).await?;
            }
        } else {
            channel
                .write(&GetBlock::new(sync_handler.block_headers.remove(0)))
                .await?;
        }
    }

    Ok(())
}

pub async fn process_transaction_internal(
    context: Arc<Context>,
    storage: Arc<BlockStorage>,
    memory_pool_lock: Arc<Mutex<MemoryPool>>,
    transaction_bytes: Vec<u8>,
    transaction_sender: SocketAddr,
) -> Result<(), SendError> {
    if let Ok(transaction) = TransactionStruct::deserialize(&transaction_bytes) {
        let mut memory_pool = memory_pool_lock.lock().await;

        let entry = Entry {
            size: transaction_bytes.len(),
            transaction,
        };

        if let Ok(inserted) = memory_pool.insert(&storage, entry) {
            if inserted.is_some() {
                info!("Transaction added to mempool. Propagating transaction to peers");

                for (socket, _) in &context.peer_book.read().await.peers.addresses {
                    if *socket != transaction_sender && *socket != context.local_addr {
                        if let Some(channel) = context.connections.read().await.get(socket) {
                            channel.write(&Transaction::new(transaction_bytes.clone())).await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Announce block to peers
pub async fn propagate_block(context: Arc<Context>, data: Vec<u8>, block_miner: SocketAddr) -> Result<(), SendError> {
    info!("Propagating block to peers");

    for (socket, _) in &context.peer_book.read().await.peers.addresses {
        if *socket != block_miner && *socket != context.local_addr {
            if let Some(channel) = context.connections.read().await.get(socket) {
                channel.write(&Block::new(data.clone())).await?;
            }
        }
    }
    Ok(())
}
