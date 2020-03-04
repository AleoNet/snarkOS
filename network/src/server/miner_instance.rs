use crate::{context::Context, server::propagate_block};
use snarkos_consensus::{
    miner::{MemoryPool, Miner},
    ConsensusParameters,
};
use snarkos_objects::Block;
use snarkos_storage::BlockStorage;

use std::sync::Arc;
use tokio::{sync::Mutex, task};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

/// Parameters for spawning a miner that runs proof of work to find a block.
pub struct MinerInstance {
    pub coinbase_address: BitcoinAddress<Mainnet>,
    pub consensus: ConsensusParameters,
    pub storage: Arc<BlockStorage>,
    pub memory_pool_lock: Arc<Mutex<MemoryPool>>,
    pub server_context: Arc<Context>,
}

impl MinerInstance {
    /// Creates a new MinerInstance for spawning miners.
    pub fn new(
        coinbase_address: BitcoinAddress<Mainnet>,
        consensus: ConsensusParameters,
        storage: Arc<BlockStorage>,
        memory_pool_lock: Arc<Mutex<MemoryPool>>,
        server_context: Arc<Context>,
    ) -> Self {
        Self {
            coinbase_address,
            consensus,
            storage,
            memory_pool_lock,
            server_context,
        }
    }

    /// Spawns a new miner on a new thread using MinerInstance parameters.
    ///
    /// Once a block is found, A block message is sent to all peers.
    /// Calling this function multiple times will spawn additional listeners on separate threads.
    /// Miner threads are asynchronous so the only way to stop them is to kill the runtime they were started in. This may be changed in the future.
    pub fn spawn(self) {
        task::spawn(async move {
            let context = self.server_context.clone();
            let local_address = self.server_context.local_address;
            let miner = Miner::new(self.coinbase_address.clone(), self.consensus.clone());

            loop {
                let block_serialized = miner.mine_block(&self.storage, &self.memory_pool_lock).await.unwrap();

                info!(
                    "Block found!           {:?}",
                    Block::deserialize(&block_serialized).unwrap()
                );

                propagate_block(context.clone(), block_serialized, local_address)
                    .await
                    .unwrap();
            }
        });
    }
}
