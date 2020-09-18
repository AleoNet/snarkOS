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

use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger, Miner};
use snarkos_dpc::base_dpc::{instantiated::*, parameters::PublicParameters};
use snarkos_network::server::{context::Context, propagate_block};
use snarkos_objects::{AccountAddress, Block};

use std::sync::Arc;
use tokio::{sync::Mutex, task};

/// Parameters for spawning a miner that runs proof of work to find a block.
pub struct MinerInstance {
    miner_address: AccountAddress<Components>,
    consensus: ConsensusParameters,
    parameters: PublicParameters<Components>,
    storage: Arc<MerkleTreeLedger>,
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    server_context: Arc<Context>,
}

impl MinerInstance {
    /// Creates a new MinerInstance for spawning miners.
    pub fn new(
        miner_address: AccountAddress<Components>,
        consensus: ConsensusParameters,
        parameters: PublicParameters<Components>,
        storage: Arc<MerkleTreeLedger>,
        memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
        server_context: Arc<Context>,
    ) -> Self {
        Self {
            miner_address,
            consensus,
            parameters,
            storage,
            memory_pool_lock,
            server_context,
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
            info!("Initializing Aleo miner - Your miner address is {}", self.miner_address);
            let miner = Miner::new(self.miner_address.clone(), self.consensus.clone());

            let mut mining_failure_count = 0;
            let mining_failure_threshold = 10;

            loop {
                info!("Starting to mine the next block");

                let (block_serialized, _coinbase_records) = match miner
                    .mine_block(&self.parameters, &self.storage, &self.memory_pool_lock)
                    .await
                {
                    Ok(mined_block) => mined_block,
                    Err(error) => {
                        warn!(
                            "Miner failed to mine a block {} time(s). (error message: {}).",
                            mining_failure_count, error
                        );
                        mining_failure_count += 1;

                        if mining_failure_count >= mining_failure_threshold {
                            warn!(
                                "Miner has failed to mine a block {} times. Shutting down miner.",
                                mining_failure_count
                            );
                            break;
                        } else {
                            continue;
                        }
                    }
                };

                match Block::<Tx>::deserialize(&block_serialized) {
                    Ok(block) => {
                        info!("Mined a new block!\t{:?}", hex::encode(block.header.get_hash().0));

                        if let Err(err) = propagate_block(context.clone(), block_serialized, local_address).await {
                            error!("Error propagating block to peers: {:?}", err);
                        }
                    }
                    Err(_) => continue,
                }
            }
        });
    }
}
