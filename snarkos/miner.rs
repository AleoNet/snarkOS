// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use snarkos_consensus::Miner;
use snarkos_network::{environment::Environment, Server as NodeServer};
use snarkvm_dpc::base_dpc::instantiated::*;
use snarkvm_objects::AccountAddress;

use tokio::task;
use tracing::*;

use std::sync::Arc;

/// Parameters for spawning a miner that runs proof of work to find a block.
pub struct MinerInstance {
    miner_address: AccountAddress<Components>,
    environment: Environment,
    node_server: NodeServer,
}

impl MinerInstance {
    /// Creates a new MinerInstance for spawning miners.
    pub fn new(miner_address: AccountAddress<Components>, environment: Environment, node_server: NodeServer) -> Self {
        Self {
            miner_address,
            environment,
            node_server,
        }
    }

    /// Spawns a new miner on a new thread using MinerInstance parameters.
    /// Once a block is found, A block message is sent to all peers.
    /// Calling this function multiple times will spawn additional listeners on separate threads.
    /// Miner threads are asynchronous so the only way to stop them is to kill the runtime they were started in. This may be changed in the future.
    pub fn spawn(self) {
        task::spawn(async move {
            let local_address = self.environment.local_address().unwrap();
            info!("Initializing Aleo miner - Your miner address is {}", self.miner_address);
            let miner = Miner::new(
                self.miner_address.clone(),
                Arc::clone(self.environment.consensus_parameters()),
            );
            info!("Miner instantiated; starting to mine blocks");

            let mut mining_failure_count = 0;
            let mining_failure_threshold = 10;

            loop {
                info!("Starting to mine the next block");

                let (block, _coinbase_records) = match miner
                    .mine_block(
                        self.environment.dpc_parameters(),
                        self.environment.storage(),
                        self.environment.memory_pool(),
                    )
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

                info!("Mined a new block!\t{:?}", hex::encode(block.header.get_hash().0));
                let peers = self.node_server.peers.connected_peers();
                let serialized_block = if let Ok(block) = block.serialize() {
                    block
                } else {
                    error!("Our own miner baked an unserializable block!");
                    continue;
                };

                self.node_server
                    .blocks
                    .propagate_block(serialized_block, local_address, &peers)
                    .await;
            }
        });
    }
}
