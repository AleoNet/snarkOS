use crate::{context::Context, server::propagate_block};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger, Miner};
use snarkos_dpc::base_dpc::{instantiated::*, parameters::PublicParameters};
use snarkos_objects::{AccountPublicKey, Block};

use std::sync::Arc;
use tokio::{sync::Mutex, task};

/// Parameters for spawning a miner that runs proof of work to find a block.
pub struct MinerInstance {
    miner_address: AccountPublicKey<Components>,
    consensus: ConsensusParameters,
    parameters: PublicParameters<Components>,
    storage: Arc<MerkleTreeLedger>,
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    server_context: Arc<Context>,
    network_id: u8,
}

impl MinerInstance {
    /// Creates a new MinerInstance for spawning miners.
    pub fn new(
        miner_address: AccountPublicKey<Components>,
        consensus: ConsensusParameters,
        parameters: PublicParameters<Components>,
        storage: Arc<MerkleTreeLedger>,
        memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
        server_context: Arc<Context>,
        network_id: u8,
    ) -> Self {
        Self {
            miner_address,
            consensus,
            parameters,
            storage,
            memory_pool_lock,
            server_context,
            network_id,
        }
    }

    /// Spawns a new miner on a new thread using MinerInstance parameters.
    /// Once a block is found, A block message is sent to all peers.
    /// Calling this function multiple times will spawn additional listeners on separate threads.
    /// Miner threads are asynchronous so the only way to stop them is to kill the runtime they were started in. This may be changed in the future.
    pub fn spawn(self) {
        task::spawn(async move {
            let context = self.server_context.clone();
            let local_address = *self.server_context.local_address.read().await;
            let miner = Miner::new(self.miner_address.clone(), self.consensus.clone());

            loop {
                info!("Mining new block");

                let (block_serialized, _coinbase_records) = miner
                    .mine_block(&self.parameters, &self.storage, &self.memory_pool_lock, self.network_id)
                    .await
                    .unwrap();

                match Block::<Tx>::deserialize(&block_serialized) {
                    Ok(block) => {
                        info!("Block found!    {:?}", block.header.get_hash());

                        if let Err(err) = propagate_block(context.clone(), block_serialized, local_address).await {
                            info!("Error propagating block to peers: {:?}", err);
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
    }
}
