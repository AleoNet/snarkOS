use crate::{server::propagate_block, Server};
use snarkos_consensus::miner::Miner;
use snarkos_objects::Block;
use snarkos_utilities::unwrap_result_or_continue;

use std::time::Duration;
use tokio::{task, time::delay_for};
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

impl Server {
    /// Spawns a new miner on a new thread using MinerInstance parameters.
    /// Once a block is found, A block message is sent to all peers.
    /// Calling this function multiple times will spawn additional miners on separate threads.
    /// Miner threads are asynchronous so the only way to stop them is to kill the runtime they were started in. This may be changed in the future.
    pub fn start_miner(&self, coinbase_address: BitcoinAddress<Mainnet>) {
        let local_address = self.context.local_address;
        let context = self.context.clone();
        let storage = self.storage.clone();
        let memory_pool_lock = self.memory_pool_lock.clone();
        let miner = Miner::new(coinbase_address, self.consensus.clone());

        task::spawn(async move {
            loop {
                // This time delay ensures that the thread does not attempt to acquire locks too aggressively
                delay_for(Duration::from_millis(100)).await;

                let block_serialized = unwrap_result_or_continue!(miner.mine_block(&storage, &memory_pool_lock).await);

                info!(
                    "Block found!           {:?}",
                    Block::deserialize(&block_serialized).expect("Unable to deserialize mined block")
                );

                propagate_block(context.clone(), block_serialized, local_address)
                    .await
                    .unwrap_or_else(|error| info!("Miner failed to propagate block {}", error));
            }
        });
    }
}
