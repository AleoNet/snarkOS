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

use super::*;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use snarkos_node_env::ENV_INFO;
use snarkvm::prelude::{Identifier, Plaintext, Transaction};

/// The `get_blocks` query object.
#[derive(Deserialize, Serialize)]
pub(crate) struct BlockRange {
    /// The starting block height (inclusive).
    start: u32,
    /// The ending block height (exclusive).
    end: u32,
}

impl<N: Network, C: ConsensusStorage<N>, R: Routing<N>> Rest<N, C, R> {
    // GET /testnet3/latest/height
    pub(crate) async fn latest_height(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_height())
    }

    // GET /testnet3/latest/hash
    pub(crate) async fn latest_hash(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_hash())
    }

    // GET /testnet3/latest/block
    pub(crate) async fn latest_block(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_block())
    }

    // GET /testnet3/latest/stateRoot
    pub(crate) async fn latest_state_root(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.ledger.latest_state_root())
    }

    // GET /testnet3/block/{height}
    // GET /testnet3/block/{blockHash}
    pub(crate) async fn get_block(
        State(rest): State<Self>,
        Path(height_or_hash): Path<String>,
    ) -> Result<ErasedJson, RestError> {
        // Manually parse the height or the height or the hash, axum doesn't support different types
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

    // GET /testnet3/blocks?start={start_height}&end={end_height}
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

        let blocks = cfg_into_iter!((start_height..end_height))
            .map(|height| rest.ledger.get_block(height))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ErasedJson::pretty(blocks))
    }

    // GET /testnet3/height/{blockHash}
    pub(crate) async fn get_height(
        State(rest): State<Self>,
        Path(hash): Path<N::BlockHash>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_height(&hash)?))
    }

    // GET /testnet3/block/{height}/transactions
    pub(crate) async fn get_block_transactions(
        State(rest): State<Self>,
        Path(height): Path<u32>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_transactions(height)?))
    }

    // GET /testnet3/transaction/{transactionID}
    pub(crate) async fn get_transaction(
        State(rest): State<Self>,
        Path(tx_id): Path<N::TransactionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_transaction(tx_id)?))
    }

    // GET /testnet3/memoryPool/transactions
    pub(crate) async fn get_memory_pool_transactions(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        match rest.consensus {
            Some(consensus) => Ok(ErasedJson::pretty(consensus.memory_pool().unconfirmed_transactions())),
            None => Err(RestError("route isn't available for this node type".to_string())),
        }
    }

    // GET /testnet3/program/{programID}
    pub(crate) async fn get_program(
        State(rest): State<Self>,
        Path(id): Path<ProgramID<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_program(id)?))
    }

    // GET /testnet3/program/{programID}/mappings
    pub(crate) async fn get_mapping_names(
        State(rest): State<Self>,
        Path(id): Path<ProgramID<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.vm().finalize_store().get_mapping_names_speculative(&id)?))
    }

    // GET /testnet3/program/{programID}/mapping/{mappingName}/{mappingKey}
    pub(crate) async fn get_mapping_value(
        State(rest): State<Self>,
        Path((id, name, key)): Path<(ProgramID<N>, Identifier<N>, Plaintext<N>)>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.vm().finalize_store().get_value_speculative(
            &id,
            &name,
            &key,
        )?))
    }

    // GET /testnet3/statePath/{commitment}
    pub(crate) async fn get_state_path_for_commitment(
        State(rest): State<Self>,
        Path(commitment): Path<Field<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.get_state_path_for_commitment(&commitment)?))
    }

    // GET /testnet3/beacons
    pub(crate) async fn get_beacons(State(rest): State<Self>) -> Result<ErasedJson, RestError> {
        match rest.consensus {
            Some(consensus) => Ok(ErasedJson::pretty(consensus.ledger().latest_committee())),
            None => Err(RestError("route isn't available for this node type".to_string())),
        }
    }

    // GET /testnet3/peers/count
    pub(crate) async fn get_peers_count(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().number_of_connected_peers())
    }

    // GET /testnet3/peers/all
    pub(crate) async fn get_peers_all(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().connected_peers())
    }

    // GET /testnet3/peers/all/metrics
    pub(crate) async fn get_peers_all_metrics(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().connected_metrics())
    }

    // GET /testnet3/node/address
    pub(crate) async fn get_node_address(State(rest): State<Self>) -> ErasedJson {
        ErasedJson::pretty(rest.routing.router().address())
    }

    // GET /testnet3/find/blockHash/{transactionID}
    pub(crate) async fn find_block_hash(
        State(rest): State<Self>,
        Path(tx_id): Path<N::TransactionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_block_hash(&tx_id)?))
    }

    /*
    // GET /testnet3/find/mappingValue/{mappingKey}
    pub(crate) async fn find_mapping_key(
        State(rest): State<Self>,
        Path(mapping_key): Path<Field<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.vm().finalize_store().get_value_from_key_id_speculative(&mapping_key)?))
    }
     */

    // GET /testnet3/find/transactionID/deployment/{programID}
    pub(crate) async fn find_transaction_id_from_program_id(
        State(rest): State<Self>,
        Path(program_id): Path<ProgramID<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_transaction_id_from_program_id(&program_id)?))
    }

    // GET /testnet3/find/transactionID/{transitionID}
    pub(crate) async fn find_transaction_id_from_transition_id(
        State(rest): State<Self>,
        Path(transition_id): Path<N::TransitionID>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_transaction_id_from_transition_id(&transition_id)?))
    }

    // GET /testnet3/find/transitionID/{inputOrOutputID}
    pub(crate) async fn find_transition_id(
        State(rest): State<Self>,
        Path(input_or_output_id): Path<Field<N>>,
    ) -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(rest.ledger.find_transition_id(&input_or_output_id)?))
    }

    // GET /testnet3/node/env
    pub(crate) async fn get_env_info() -> Result<ErasedJson, RestError> {
        Ok(ErasedJson::pretty(ENV_INFO.get()))
    }

    // POST /testnet3/transaction/broadcast
    pub(crate) async fn transaction_broadcast(
        State(rest): State<Self>,
        Json(tx): Json<Transaction<N>>,
    ) -> Result<ErasedJson, RestError> {
        // If the consensus module is enabled, add the unconfirmed transaction to the memory pool.
        if let Some(consensus) = rest.consensus {
            // Add the unconfirmed transaction to the memory pool.
            consensus.add_unconfirmed_transaction(tx.clone())?;
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
}
