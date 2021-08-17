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

use std::{sync::Arc, thread, time::Duration};

use futures::executor::block_on;
use snarkvm_dpc::{testnet1::instantiated::*, Address};
use tokio::task;
use tracing::*;

use snarkos_consensus::{error::ConsensusError, MineContext};
use snarkos_metrics::{self as metrics, misc::*};

use crate::{Node, State};

/// Parameters for spawning a miner that runs proof of work to find a block.
pub struct MinerInstance {
    miner_address: Address<Components>,
    node: Node,
}

impl MinerInstance {
    /// Creates a new MinerInstance for spawning miners.
    pub fn new(miner_address: Address<Components>, node: Node) -> Self {
        Self { miner_address, node }
    }

    /// Spawns a new miner on a new thread using MinerInstance parameters.
    /// Once a block is found, A block message is sent to all peers.
    /// Calling this function multiple times will spawn additional listeners on separate threads.
    pub async fn spawn(self) -> Result<task::JoinHandle<()>, ConsensusError> {
        let local_address = self.node.local_address().unwrap();
        info!("Initializing Aleo miner - Your miner address is {}", self.miner_address);
        let miner = MineContext::prepare(
            self.miner_address.clone(),
            Arc::clone(&self.node.expect_sync().consensus),
        )
        .await?;
        info!("Miner instantiated; starting to mine blocks");

        let mut mining_failure_count = 0;
        let mining_failure_threshold = 10;

        Ok(task::spawn_blocking(move || {
            loop {
                if self.node.is_shutting_down() {
                    debug!("The node is shutting down, stopping mining");
                    break;
                }

                // Don't mine if the node is currently syncing.
                if self.node.state() == State::Syncing {
                    thread::sleep(Duration::from_secs(15));
                    continue;
                } else {
                    self.node.set_state(State::Mining);
                }

                info!("Starting to mine the next block");

                let (block, _coinbase_records) = match block_on(miner.mine_block()) {
                    Ok(mined_block) => mined_block,
                    Err(error) => {
                        // It's possible that the node realized that it needs to sync with another one in the
                        // meantime; don't change to `Idle` if the current status isn't still `Mining`.
                        if self.node.state() == State::Mining {
                            self.node.set_state(State::Idle);
                        }

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

                // See the `Err` path note above.
                if self.node.state() == State::Mining {
                    self.node.set_state(State::Idle);
                }

                metrics::increment_counter!(BLOCKS_MINED);

                info!("Mined a new block: {:?}", hex::encode(block.header.hash().0));

                let serialized_block = block.serialize();
                let node_clone = self.node.clone();
                let new_height = futures::executor::block_on(async move {
                    node_clone.storage.canon().await.map(|c| c.block_height as u32)
                })
                .ok();

                self.node.propagate_block(serialized_block, new_height, local_address);
            }
        }))
    }
}
