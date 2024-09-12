// Copyright 2024 Aleo Network Foundation
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

use super::*;
use snarkos_node_router::{messages::UnconfirmedSolution, SYNC_LENIENCY};
use snarkvm::{
    ledger::puzzle::Solution,
    prelude::{block::Transaction, Address, Identifier, LimitedWriter, Plaintext, ToBytes},
};

use indexmap::IndexMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// The `get_blocks` query object.
#[derive(Deserialize, Serialize)]
pub(crate) struct BlockRange {
    /// The starting block height (inclusive).
    start: u32,
    /// The ending block height (exclusive).
    end: u32,
}

/// The `get_mapping_value` query object.
#[derive(Deserialize, Serialize)]
pub(crate) struct Metadata {
    metadata: bool,
}

impl<N: Network, C: ConsensusStorage<N>, R: Routing<N>> Rest<N, C, R> {
    // ----------------- DEPRECATED FUNCTIONS -----------------
    // The functions below are associated with deprecated routes.
    // Please use the recommended alternatives when implementing new features or refactoring.

    // Deprecated: Use `get_block_height_latest` instead.
    // GET /<network>/latest/height
    pub(crate) async fn latest_height(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_height())
    }

    // Deprecated: Use `get_block_hash_latest` instead.
    // GET /<network>/latest/hash
    pub(crate) async fn latest_hash(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_hash())
    }

    // Deprecated: Use `get_block_latest` instead.
    // GET /<network>/latest/block
    pub(crate) async fn latest_block(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_block())
    }

    // Deprecated: Use `get_state_root_latest` instead.
    // GET /<network>/latest/stateRoot
    pub(crate) async fn latest_state_root(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_state_root())
    }

    // Deprecated: Use `get_committee_latest` instead.
    // GET /<network>/latest/committee
    pub(crate) async fn latest_committee(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.latest_committee()?))
    }

    // ---------------------------------------------------------

    // GET /<network>/block/height/latest
    pub(crate) async fn get_block_height_latest(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_height())
    }

    // GET /<network>/block/hash/latest
    pub(crate) async fn get_block_hash_latest(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_hash())
    }

    // GET /<network>/block/latest
    pub(crate) async fn get_block_latest(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_block())
    }

    // GET /<network>/block/{height}
    // GET /<network>/block/{blockHash}
    pub(crate) async fn get_block(
        State(rest): State<Self>,
        Path(height_or_hash): Path<String>,
    ) -> Result<ErasedJson, RestError> {
        // Manually parse the height or the height of the hash, axum doesn't support different types
        // for the same path param.
        let block = if let Ok(height) = height_or_hash.parse::<u32>() {
            rest.ledger.get_block(height)?
        } else {
            let hash = height_or_hash
                .parse::<N::BlockHash>()
                .map_err(|_| RestError("invalid input, it is neither a block height nor a block hash".to_string()))?;

            rest.ledger.get_block_by_hash(&hash)?
        };

        Ok(ErasedJson::pretty(block))
    }

    // GET /<network>/blocks?start={start_height}&end={end_height}
    pub(crate) async fn get_blocks(
        State(rest): State<Self>,
        Query(block_range): Query<BlockRange>,
    ) -> Result<ErasedJson, RestError> {
        let start_height = block_range.start;
        let end_height = block_range.end;

        const MAX_BLOCK_RANGE: u32 = 50;

        // Ensure the end height is greater than the start height.
        if start_height > end_height {
            return Err(RestError("Invalid block range".to_string()));
        }

        // Ensure the block range is bounded.
        if end_height - start_height > MAX_BLOCK_RANGE {
            return Err(RestError(format!(
                "Cannot request more than {MAX_BLOCK_RANGE} blocks per call (requested {})",
                end_height - start_height
            )));
        }

        // Prepare a closure for the blocking work.
        let get_json_blocks = move || -> Result<ErasedJson, RestError> {
            let blocks = cfg_into_iter!((start_height..end_height))
                .map(|height| rest.ledger.get_block(height))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ErasedJson::pretty(blocks))
        };

        // Fetch the blocks from ledger and serialize to json.
        match tokio::task::spawn_blocking(get_json_blocks).await {
            Ok(json) => json,
            Err(err) => Err(RestError(format!("Failed to get blocks '{start_height}..{end_height}' - {err}"))),
        }
    }

    // GET /<network>/height/{blockHash}
    pub(crate) async fn get_height(
        State(rest): State<Self>,
        Path(hash): Path<N::BlockHash>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_height(&hash)?))
    }

    // GET /<network>/block/{height}/transactions
    pub(crate) async fn get_block_transactions(
        State(rest): State<Self>,
        Path(height): Path<u32>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_transactions(height)?))
    }

    // GET /<network>/transaction/{transactionID}
    pub(crate) async fn get_transaction(
        State(rest): State<Self>,
        Path(tx_id): Path<N::TransactionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_transaction(tx_id)?))
    }

    // GET /<network>/transaction/confirmed/{transactionID}
    pub(crate) async fn get_confirmed_transaction(
        State(rest): State<Self>,
        Path(tx_id): Path<N::TransactionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_confirmed_transaction(tx_id)?))
    }

    // GET /<network>/memoryPool/transmissions
    pub(crate) async fn get_memory_pool_transmissions(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        match rest.consensus {
            Some(consensus) => {
                Ok(ErasedJson::pretty(consensus.unconfirmed_transmissions().collect::<IndexMap<_, _>>()))
            }
            None => Err(RestError("Route isn't available for this node type".to_string())),
        }
    }

    // GET /<network>/memoryPool/solutions
    pub(crate) async fn get_memory_pool_solutions(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        match rest.consensus {
            Some(consensus) => Ok(ErasedJson::pretty(consensus.unconfirmed_solutions().collect::<IndexMap<_, _>>())),
            None => Err(RestError("Route isn't available for this node type".to_string())),
        }
    }

    // GET /<network>/memoryPool/transactions
    pub(crate) async fn get_memory_pool_transactions(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        match rest.consensus {
            Some(consensus) => Ok(ErasedJson::pretty(consensus.unconfirmed_transactions().collect::<IndexMap<_, _>>())),
            None => Err(RestError("Route isn't available for this node type".to_string())),
        }
    }

    // GET /<network>/program/{programID}
    pub(crate) async fn get_program(
        State(rest): State<Self>,
        Path(id): Path<ProgramID<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_program(id)?))
    }

    // GET /<network>/program/{programID}/mappings
    pub(crate) async fn get_mapping_names(
        State(rest): State<Self>,
        Path(id): Path<ProgramID<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.vm().finalize_store().get_mapping_names_confirmed(&id)?))
    }

    // GET /<network>/program/{programID}/mapping/{mappingName}/{mappingKey}
    // GET /<network>/program/{programID}/mapping/{mappingName}/{mappingKey}?metadata={true}
    pub(crate) async fn get_mapping_value(
        State(rest): State<Self>,
        Path((id, name, key)): Path<(ProgramID<N>, Identifier<N>, Plaintext<N>)>,
        metadata: Option<Query<Metadata>>,
    ) -> Result<ErasedJson, RestError> {
        // Retrieve the mapping value.
        let mapping_value = rest.ledger.vm().finalize_store().get_value_confirmed(id, name, &key)?;

        // Check if metadata is requested and return the value with metadata if so.
        if metadata.map(|q| q.metadata).unwrap_or(false) {
            return Ok(ErasedJson::pretty(json!({
                "data": mapping_value,
                "height": rest.ledger.latest_height(),
            })));
        }

        // Return the value without metadata.
        Ok(ErasedJson::pretty(mapping_value))
    }

    // GET /<network>/statePath/{commitment}
    pub(crate) async fn get_state_path_for_commitment(
        State(rest): State<Self>,
        Path(commitment): Path<Field<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_state_path_for_commitment(&commitment)?))
    }

    // GET /<network>/stateRoot/latest
    pub(crate) async fn get_state_root_latest(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_state_root())
    }

    // GET /<network>/stateRoot/{height}
    pub(crate) async fn get_state_root(
        State(rest): State<Self>,
        Path(height): Path<u32>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_state_root(height)?))
    }

    // GET /<network>/committee/latest
    pub(crate) async fn get_committee_latest(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.latest_committee()?))
    }

    // GET /<network>/committee/{height}
    pub(crate) async fn get_committee(
        State(rest): State<Self>,
        Path(height): Path<u32>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_committee(height)?))
    }

    // GET /<network>/delegators/{validator}
    pub(crate) async fn get_delegators_for_validator(
        State(rest): State<Self>,
        Path(validator): Path<Address<N>>,
    ) -> Result<ErasedJson, RestError> {
        // Do not process the request if the node is too far behind to avoid sending outdated data.
        if rest.routing.num_blocks_behind() > SYNC_LENIENCY {
            return Err(RestError("Unable to  request delegators (node is syncing)".to_string()));
        }

        // Return the delegators for the given validator.
        match tokio::task::spawn_blocking(move || rest.ledger.get_delegators_for_validator(&validator)).await {
            Ok(Ok(delegators)) => Ok(ErasedJson::pretty(delegators)),
            Ok(Err(err)) => Err(RestError(format!("Unable to request delegators - {err}"))),
            Err(err) => Err(RestError(format!("Unable to request delegators - {err}"))),
        }
    }

    // GET /<network>/peers/count
    pub(crate) async fn get_peers_count(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().number_of_connected_peers())
    }

    // GET /<network>/peers/all
    pub(crate) async fn get_peers_all(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().connected_peers())
    }

    // GET /<network>/peers/all/metrics
    pub(crate) async fn get_peers_all_metrics(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().connected_metrics())
    }

    // GET /<network>/node/address
    pub(crate) async fn get_node_address(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().address())
    }

    // GET /<network>/find/blockHash/{transactionID}
    pub(crate) async fn find_block_hash(
        State(rest): State<Self>,
        Path(tx_id): Path<N::TransactionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_block_hash(&tx_id)?))
    }

    // GET /<network>/find/blockHeight/{stateRoot}
    pub(crate) async fn find_block_height_from_state_root(
        State(rest): State<Self>,
        Path(state_root): Path<N::StateRoot>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_block_height_from_state_root(state_root)?))
    }

    // GET /<network>/find/transactionID/deployment/{programID}
    pub(crate) async fn find_transaction_id_from_program_id(
        State(rest): State<Self>,
        Path(program_id): Path<ProgramID<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_transaction_id_from_program_id(&program_id)?))
    }

    // GET /<network>/find/transactionID/{transitionID}
    pub(crate) async fn find_transaction_id_from_transition_id(
        State(rest): State<Self>,
        Path(transition_id): Path<N::TransitionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_transaction_id_from_transition_id(&transition_id)?))
    }

    // GET /<network>/find/transitionID/{inputOrOutputID}
    pub(crate) async fn find_transition_id(
        State(rest): State<Self>,
        Path(input_or_output_id): Path<Field<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_transition_id(&input_or_output_id)?))
    }

    // POST /<network>/transaction/broadcast
    pub(crate) async fn transaction_broadcast(
        State(rest): State<Self>,
        Json(tx): Json<Transaction<N>>,
    ) -> Result<ErasedJson, RestError> {
        // Do not process the transaction if the node is too far behind.
        if rest.routing.num_blocks_behind() > SYNC_LENIENCY {
            return Err(RestError(format!("Unable to broadcast transaction '{}' (node is syncing)", fmt_id(tx.id()))));
        }

        // If the transaction exceeds the transaction size limit, return an error.
        // The buffer is initially roughly sized to hold a `transfer_public`,
        // most transactions will be smaller and this reduces unnecessary allocations.
        // TODO: Should this be a blocking task?
        let buffer = Vec::with_capacity(3000);
        if tx.write_le(LimitedWriter::new(buffer, N::MAX_TRANSACTION_SIZE)).is_err() {
            return Err(RestError("Transaction size exceeds the byte limit".to_string()));
        }

        // If the consensus module is enabled, add the unconfirmed transaction to the memory pool.
        if let Some(consensus) = rest.consensus {
            // Add the unconfirmed transaction to the memory pool.
            consensus.add_unconfirmed_transaction(tx.clone()).await?;
        }

        // Prepare the unconfirmed transaction message.
        let tx_id = tx.id();
        let message = Message::UnconfirmedTransaction(UnconfirmedTransaction {
            transaction_id: tx_id,
            transaction: Data::Object(tx),
        });

        // Broadcast the transaction.
        rest.routing.propagate(message, &[]);

        Ok(ErasedJson::pretty(tx_id))
    }

    // POST /<network>/solution/broadcast
    pub(crate) async fn solution_broadcast(
        State(rest): State<Self>,
        Json(solution): Json<Solution<N>>,
    ) -> Result<ErasedJson, RestError> {
        // Do not process the solution if the node is too far behind.
        if rest.routing.num_blocks_behind() > SYNC_LENIENCY {
            return Err(RestError(format!(
                "Unable to broadcast solution '{}' (node is syncing)",
                fmt_id(solution.id())
            )));
        }

        // If the consensus module is enabled, add the unconfirmed solution to the memory pool.
        // Otherwise, verify it prior to broadcasting.
        match rest.consensus {
            // Add the unconfirmed solution to the memory pool.
            Some(consensus) => consensus.add_unconfirmed_solution(solution).await?,
            // Verify the solution.
            None => {
                // Compute the current epoch hash.
                let epoch_hash = rest.ledger.latest_epoch_hash()?;
                // Retrieve the current proof target.
                let proof_target = rest.ledger.latest_proof_target();
                // Ensure that the solution is valid for the given epoch.
                let puzzle = rest.ledger.puzzle().clone();
                // Verify the solution in a blocking task.
                match tokio::task::spawn_blocking(move || puzzle.check_solution(&solution, epoch_hash, proof_target))
                    .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(err)) => {
                        return Err(RestError(format!("Invalid solution '{}' - {err}", fmt_id(solution.id()))));
                    }
                    Err(err) => return Err(RestError(format!("Invalid solution '{}' - {err}", fmt_id(solution.id())))),
                }
            }
        }

        let solution_id = solution.id();
        // Prepare the unconfirmed solution message.
        let message =
            Message::UnconfirmedSolution(UnconfirmedSolution { solution_id, solution: Data::Object(solution) });

        // Broadcast the unconfirmed solution message.
        rest.routing.propagate(message, &[]);

        Ok(ErasedJson::pretty(solution_id))
    }

    // GET /{network}/block/{blockHeight}/history/{mapping}
    #[cfg(feature = "history")]
    pub(crate) async fn get_history(
        State(rest): State<Self>,
        Path((height, mapping)): Path<(u32, snarkvm::synthesizer::MappingName)>,
    ) -> Result<impl axum::response::IntoResponse, RestError> {
        // Retrieve the history for the given block height and variant.
        let history = snarkvm::synthesizer::History::new(N::ID, rest.ledger.vm().finalize_store().storage_mode());
        let result = history
            .load_mapping(height, mapping)
            .map_err(|_| RestError(format!("Could not load mapping '{mapping}' from block '{height}'")))?;

        Ok((StatusCode::OK, [(CONTENT_TYPE, "application/json")], result))
    }
}
