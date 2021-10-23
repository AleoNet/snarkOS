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

use crate::{network::peers::Peers, Environment, Message, Node, Status};
use snarkos_ledger::ledger::Ledger;
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use rand::thread_rng;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{sync::RwLock, task};

pub(crate) struct Miner<N: Network> {
    miner_address: Address<N>,
}

impl<N: Network> Miner<N> {
    pub(crate) fn spawn<E: Environment>(node: Node<N, E>, recipient: Address<N>) -> task::JoinHandle<()> {
        task::spawn(async move {
            loop {
                // Retrieve the status of the node.
                let status = node.status();
                // Ensure the node is not syncing or shutting down.
                if status != Status::Syncing && status != Status::ShuttingDown {
                    // Set the status of the node to mining.
                    node.set_status(Status::Mining);
                    // Start the mining process.
                    let result = Miner::mine_next_block(node.ledger(), node.peers(), recipient, &node.terminator()).await;
                    // Ensure the miner did not error.
                    if let Err(error) = result {
                        // Sleep for 10 seconds.
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        warn!("{}", error);
                    }
                }
            }
        })
    }

    /// Mines a new block and adds it to the canon blocks.
    async fn mine_next_block<E: Environment>(
        ledger: Arc<RwLock<Ledger<N>>>,
        peers: Arc<RwLock<Peers<N, E>>>,
        recipient: Address<N>,
        terminator: &AtomicBool,
    ) -> Result<()> {
        // Ensure the miner is connected to the network, in order to mine.
        if peers.read().await.num_connected_peers() == 0 {
            return Err(anyhow!("Unable to mine without at least one connected peer"));
        }

        // Mine the next block.
        let block = Self::mine(ledger.clone(), recipient, terminator).await?;

        // Ensure the miner is still connected to the network, in order to update the ledger.
        if peers.read().await.num_connected_peers() == 0 {
            return Err(anyhow!("Unable to update the ledger without at least one connected peer"));
        }

        // Attempt to add the block to the canon chain.
        ledger.write().await.add_next_block(&block)?;

        // On success, clear the memory pool of its transactions.
        ledger.write().await.memory_pool.clear_transactions();

        let latest_block_height = ledger.read().await.latest_block_height();
        debug!("Ledger advanced to block {}", latest_block_height);

        // Broadcast the new block to the peers.
        let message = Message::UnconfirmedBlock(latest_block_height, block);
        peers.write().await.broadcast(&message).await;

        Ok(())
    }

    /// Returns a `Block` upon mining a new block.
    async fn mine(ledger: Arc<RwLock<Ledger<N>>>, recipient: Address<N>, terminator: &AtomicBool) -> Result<Block<N>> {
        // Prepare the new block.
        let previous_block_hash = ledger.read().await.latest_block_hash();
        let block_height = ledger.read().await.latest_block_height() + 1;

        // Compute the block difficulty target.
        let previous_timestamp = ledger.read().await.latest_block_timestamp()?;
        let previous_difficulty_target = ledger.read().await.latest_block_difficulty_target()?;
        let block_timestamp = chrono::Utc::now().timestamp();
        let difficulty_target = Blocks::<N>::compute_difficulty_target(previous_timestamp, previous_difficulty_target, block_timestamp);

        // Construct the new block transactions.
        let amount = Block::<N>::block_reward(block_height);
        let coinbase_transaction = Transaction::<N>::new_coinbase(recipient, amount, &mut thread_rng())?;
        let transactions = Transactions::from(&[vec![coinbase_transaction], ledger.read().await.memory_pool.transactions()].concat())?;

        // Construct the ledger root.
        let ledger_root = ledger.read().await.latest_ledger_root();

        // Mine the next block.
        let block = Block::mine(
            previous_block_hash,
            block_height,
            block_timestamp,
            difficulty_target,
            ledger_root,
            transactions,
            terminator,
            &mut thread_rng(),
        )?;
        debug!("Miner found block {}", block.height());
        Ok(block)
    }
}
