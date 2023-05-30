// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::bail;
use async_trait::async_trait;
use bytes::BytesMut;
use narwhal_crypto::PublicKey;
use narwhal_executor::ExecutionState;
use narwhal_types::ConsensusOutput;
use parking_lot::Mutex;
use tracing::*;

use snarkos_node_consensus::Consensus as AleoConsensus;
use snarkos_node_messages::{Data, Message, NewBlock};
use snarkos_node_router::Router;
use snarkos_node_tcp::protocols::Writing;
use snarkvm::prelude::{ConsensusStorage, Network};

// The state available to the BFT consensus.
#[derive(Clone)]
pub struct BftExecutionState<N: Network, C: ConsensusStorage<N>> {
    primary_pub: PublicKey,
    router: Router<N>,
    consensus: AleoConsensus<N, C>,
    pub last_output: Arc<Mutex<Option<ConsensusOutput>>>,
    initial_last_executed_sub_dag_index: u64,
}

impl<N: Network, C: ConsensusStorage<N>> BftExecutionState<N, C> {
    pub fn new(
        primary_pub: PublicKey,
        router: Router<N>,
        consensus: AleoConsensus<N, C>,
        initial_last_executed_sub_dag_index: u64,
    ) -> Self {
        Self { primary_pub, router, consensus, last_output: Default::default(), initial_last_executed_sub_dag_index }
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> ExecutionState for BftExecutionState<N, C> {
    /// Receive the consensus result with the ordered transactions in `ConsensusOutupt`
    async fn handle_consensus_output(&self, consensus_output: ConsensusOutput) {
        *self.last_output.lock() = Some(consensus_output.clone());

        let leader = &consensus_output.sub_dag.leader.header.author;
        let mut leader_id = leader.to_string();
        leader_id.truncate(8);

        let mut validator_id = self.primary_pub.to_string();
        validator_id.truncate(8);

        info!(
            "Consensus (id: {}) output for round {}: {} batches, leader: {}",
            validator_id,
            consensus_output.sub_dag.leader.header.round,
            consensus_output.sub_dag.num_batches(),
            leader_id,
        );

        if consensus_output.batches.is_empty() {
            info!("There are no batches to process; not attempting to create a block.");
            return;
        }

        if self.primary_pub != *leader {
            info!("I'm not the current leader (id: {}), yielding block production.", validator_id);
            return;
        }

        info!("I'm the current leader (id: {}); producing a block.", validator_id);

        let consensus = self.consensus.clone();
        let private_key = *self.router.private_key();
        let next_block = tokio::task::spawn_blocking(move || {
            // Collect all the transactions contained in the agreed upon batches.
            let mut transactions = HashMap::new();
            for transaction in batched_transactions(&consensus_output) {
                let bytes = BytesMut::from(&transaction[..]);
                // TransactionValidator ensures that the Message can be deserialized.
                let message = Message::<N>::deserialize(bytes).unwrap();

                let unconfirmed_tx = if let Message::UnconfirmedTransaction(tx) = message {
                    tx
                } else {
                    // TransactionValidator ensures that the Message is an UnconfirmedTransaction.
                    unreachable!();
                };

                // TransactionValidator ensures that the Message can be deserialized.
                let tx = unconfirmed_tx.transaction.deserialize_blocking().unwrap();

                transactions.insert(tx.id(), tx);
            }

            // Sort the transactions by ID according to shared logic.
            let mut sorted_tx_ids = transactions.keys().copied().collect::<Vec<_>>();
            sort_transactions::<N>(&mut sorted_tx_ids);

            // Attempt to add the batched transactions to the Aleo mempool in a strict order.
            let mut num_valid_txs = 0;
            for id in &sorted_tx_ids {
                let transaction = transactions.remove(id).unwrap(); // guaranteed to be there
                // Skip invalid transactions.
                if consensus.add_unconfirmed_transaction(transaction).is_ok() {
                    num_valid_txs += 1;
                }
            }

            // Return early if there are no valid transactions.
            if num_valid_txs == 0 {
                debug!("No valid transactions in ConsensusOutput; not producing a block.");
                return Ok(None);
            }

            // Propose a new block.
            let next_block = match consensus.propose_next_block(&private_key, &mut rand::thread_rng()) {
                Ok(block) => block,
                Err(error) => bail!("Failed to propose the next block: {error}"),
            };

            // Ensure the block is a valid next block.
            if let Err(error) = consensus.check_next_block(&next_block) {
                // Clear the memory pool of all solutions and transactions.
                consensus.clear_memory_pool();
                bail!("Proposed an invalid block: {error}");
            }

            // Advance to the next block.
            match consensus.advance_to_next_block(&next_block) {
                Ok(()) => {
                    // Log the next block.
                    match serde_json::to_string_pretty(&next_block.header()) {
                        Ok(header) => info!("Block {}: {header}", next_block.height()),
                        Err(error) => info!("Block {}: (serde failed: {error})", next_block.height()),
                    }
                }
                Err(error) => {
                    // Clear the memory pool of all solutions and transactions.
                    consensus.clear_memory_pool();
                    bail!("Failed to advance to the next block: {error}");
                }
            }

            info!("Produced a block with {num_valid_txs} transactions.");

            Ok(Some(next_block))
        })
        .await;

        let next_block = match next_block.map_err(|err| err.into()) {
            Ok(Ok(Some(block))) => block,
            Ok(Ok(None)) => return,
            Ok(Err(error)) | Err(error) => {
                error!("Failed to produce a new block: {error}");
                return;
            }
        };

        // TODO(nkls): update the committee state with the new stake from the transaction.

        let next_block_round = next_block.round();
        let next_block_height = next_block.height();
        let next_block_hash = next_block.hash();

        // Serialize the block ahead of time to not do it for each peer.
        let serialized_block = match Data::Object(next_block).serialize().await {
            Ok(serialized_block) => Data::Buffer(serialized_block),
            Err(error) => unreachable!("Failed to serialize own block: {error}"),
        };

        // Prepare the block to be sent to all peers.
        let message = Message::<N>::NewBlock(NewBlock::new(
            next_block_round,
            next_block_height,
            next_block_hash,
            serialized_block,
        ));

        // Broadcast the new block.
        self.router.broadcast(message).unwrap();
    }

    async fn last_executed_sub_dag_index(&self) -> u64 {
        self.last_output
            .lock()
            .as_ref()
            .map(|lco| lco.sub_dag.sub_dag_index)
            .unwrap_or(self.initial_last_executed_sub_dag_index)
    }
}

// A sorting logic shared among all the validators.
pub fn sort_transactions<N: Network>(transaction_ids: &mut [N::TransactionID]) {
    // TODO: possibly sort using a more elaborate logic
    transaction_ids.sort_by_key(|id| id.to_string());
}

// Return an iterator over deduplicated transactions agreed upon by the consensus.
pub fn batched_transactions(consensus_output: &ConsensusOutput) -> impl Iterator<Item = &Vec<u8>> {
    let deduplicated_txs = consensus_output
        .batches
        .iter()
        .flat_map(|batches| batches.1.iter().flat_map(|batch| batch.transactions.iter()))
        .collect::<HashSet<_>>();
    deduplicated_txs.into_iter()
}
