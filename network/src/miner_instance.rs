use snarkos_consensus::{
    miner::{MemoryPool, Miner},
    ConsensusParameters,
};
use snarkos_objects::Block;
use snarkos_storage::BlockStorage;

use std::{net::SocketAddr, sync::Arc};
use tokio::{sync::Mutex, task};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

use crate::base::send_propagate_block;

pub struct MinerInstance {
    pub coinbase_address: BitcoinAddress<Mainnet>,
    pub consensus: ConsensusParameters,
    pub storage: Arc<BlockStorage>,
    pub memory_pool_lock: Arc<Mutex<MemoryPool>>,
    pub server_addr: SocketAddr,
}

impl MinerInstance {
    /// Creates a new MinerInstance for spawning miners
    pub fn new(
        coinbase_address: BitcoinAddress<Mainnet>,
        consensus: ConsensusParameters,
        storage: Arc<BlockStorage>,
        memory_pool_lock: Arc<Mutex<MemoryPool>>,
        server_addr: SocketAddr,
    ) -> Self {
        Self {
            coinbase_address,
            consensus,
            storage,
            memory_pool_lock,
            server_addr,
        }
    }

    /// Spawns a new Miner on a new thread using MinerInstance parameters.
    /// - Starts mining when miner_lock is set to true.
    /// - Calling this function multiple times will spawn additional listeners on separate threads.
    ///   - Modifying miner_lock will start or stop all additional listeners.
    pub fn spawn(self) {
        task::spawn(async move {
            let miner = Miner::new(self.coinbase_address.clone(), self.consensus.clone());
            loop {
                let block_serialized = miner.mine_block(&self.storage, &self.memory_pool_lock).await.unwrap();

                info!(
                    "Block found!           {:?}",
                    Block::deserialize(&block_serialized).unwrap()
                );

                send_propagate_block(self.server_addr, block_serialized).await.unwrap();
            }
        });
    }
}
