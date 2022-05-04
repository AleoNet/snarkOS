// Copyright (C) 2019-2022 Aleo Systems Inc.
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

//! Implementation of public RPC endpoints.
//!
//! See [RpcFunctions](../trait.RpcFunctions.html) for documentation of public endpoints.

use crate::{RpcContext, RpcError, RpcFunctions};
use snarkos_environment::Environment;
use snarkos_network::ProverRequest;
use snarkvm::{
    dpc::{Address, AleoAmount, Block, BlockHeader, Blocks, Network, Record, Transaction, Transactions, Transition},
    utilities::{FromBytes, ToBytes},
};

use rayon::iter::ParallelIterator;
use serde_json::Value;
use time::OffsetDateTime;

use std::{cmp::max, net::SocketAddr};

#[async_trait::async_trait]
impl<N: Network, E: Environment> RpcFunctions<N> for RpcContext<N, E> {
    /// Returns the latest block from the canonical chain.
    async fn latest_block(&self) -> Result<Block<N>, RpcError> {
        Ok(self.ledger().latest_block())
    }

    /// Returns the latest block height from the canonical chain.
    async fn latest_block_height(&self) -> Result<u32, RpcError> {
        Ok(self.ledger().latest_block_height())
    }

    /// Returns the latest cumulative weight from the canonical chain.
    async fn latest_cumulative_weight(&self) -> Result<u128, RpcError> {
        Ok(self.ledger().latest_cumulative_weight())
    }

    /// Returns the latest block hash from the canonical chain.
    async fn latest_block_hash(&self) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger().latest_block_hash())
    }

    /// Returns the latest block header from the canonical chain.
    async fn latest_block_header(&self) -> Result<BlockHeader<N>, RpcError> {
        Ok(self.ledger().latest_block_header())
    }

    /// Returns the latest block transactions from the canonical chain.
    async fn latest_block_transactions(&self) -> Result<Transactions<N>, RpcError> {
        Ok(self.ledger().latest_block_transactions())
    }

    /// Returns the latest ledger root from the canonical chain.
    async fn latest_ledger_root(&self) -> Result<N::LedgerRoot, RpcError> {
        Ok(self.ledger().latest_ledger_root())
    }

    /// Returns the block given the block height.
    async fn get_block(&self, block_height: u32) -> Result<Block<N>, RpcError> {
        Ok(self.ledger().get_block(block_height)?)
    }

    /// Returns up to `MAXIMUM_BLOCK_REQUEST` blocks from the given `start_block_height` to `end_block_height` (inclusive).
    async fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>, RpcError> {
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        Ok(self
            .ledger()
            .get_blocks(safe_start_height, end_block_height)?
            .collect::<Result<Vec<Block<N>>, _>>()?)
    }

    /// Returns the block height for the given the block hash.
    async fn get_block_height(&self, block_hash: N::BlockHash) -> Result<u32, RpcError> {
        Ok(self.ledger().get_block_height(&block_hash)?)
    }

    /// Returns the block hash for the given block height, if it exists in the canonical chain.
    async fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger().get_block_hash(block_height)?)
    }

    /// Returns up to `MAXIMUM_BLOCK_REQUEST` block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    async fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>, RpcError> {
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        Ok(self.ledger().get_block_hashes(safe_start_height, end_block_height)?)
    }

    /// Returns the block header for the given the block height.
    async fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>, RpcError> {
        Ok(self.ledger().get_block_header(block_height)?)
    }

    /// Returns the block template for the next mined block
    async fn get_block_template(&self) -> Result<Value, RpcError> {
        // Fetch the latest state from the ledger.
        let latest_block = self.ledger().latest_block();
        let ledger_root = self.ledger().latest_ledger_root();

        // Prepare the new block.
        let previous_block_hash = latest_block.hash();
        let block_height = self.ledger().latest_block_height() + 1;
        let block_timestamp = OffsetDateTime::now_utc().unix_timestamp();

        // Compute the block difficulty target.
        let difficulty_target = if N::NETWORK_ID == 2 && block_height <= snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT {
            Blocks::<N>::compute_difficulty_target(latest_block.header(), block_timestamp, block_height)
        } else if N::NETWORK_ID == 2 {
            let anchor_block_header = self.ledger().get_block_header(snarkvm::dpc::testnet2::V12_UPGRADE_BLOCK_HEIGHT)?;
            Blocks::<N>::compute_difficulty_target(&anchor_block_header, block_timestamp, block_height)
        } else {
            Blocks::<N>::compute_difficulty_target(N::genesis_block().header(), block_timestamp, block_height)
        };

        // Compute the cumulative weight.
        let cumulative_weight = latest_block
            .cumulative_weight()
            .saturating_add((u64::MAX / difficulty_target) as u128);

        // Compute the coinbase reward (not including the transaction fees).
        let mut coinbase_reward = Block::<N>::block_reward(block_height);
        let mut transaction_fees = AleoAmount::ZERO;

        // Get and filter the transactions from the mempool.
        let transactions: Vec<String> = self
            .state
            .prover()
            .memory_pool()
            .read()
            .await
            .transactions()
            .iter()
            .filter(|transaction| {
                for serial_number in transaction.serial_numbers() {
                    if let Ok(true) = self.ledger().contains_serial_number(serial_number) {
                        return false;
                    }
                }

                for commitment in transaction.commitments() {
                    if let Ok(true) = self.ledger().contains_commitment(commitment) {
                        return false;
                    }
                }

                transaction_fees = transaction_fees.add(transaction.value_balance());
                true
            })
            .map(|tx| tx.to_string())
            .collect();

        // Enforce that the transaction fee is positive or zero.
        if transaction_fees.is_negative() {
            return Err(RpcError::Message("Invalid transaction fees".to_string()));
        }

        // Calculate the final coinbase reward (including the transaction fees).
        coinbase_reward = coinbase_reward.add(transaction_fees);

        Ok(serde_json::json!({
            "previous_block_hash": previous_block_hash,
            "block_height": block_height,
            "time": block_timestamp,
            "difficulty_target": difficulty_target,
            "cumulative_weight": cumulative_weight,
            "ledger_root": ledger_root,
            "transactions": transactions,
            "coinbase_reward": coinbase_reward,
        }))
    }

    /// Returns the transactions from the block of the given block height.
    async fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>, RpcError> {
        Ok(self.ledger().get_block_transactions(block_height)?)
    }

    /// Returns the ciphertext given the commitment.
    async fn get_ciphertext(&self, commitment: N::Commitment) -> Result<N::RecordCiphertext, RpcError> {
        Ok(self.ledger().get_ciphertext(&commitment)?)
    }

    /// Returns the ledger proof for a given record commitment.
    async fn get_ledger_proof(&self, record_commitment: N::Commitment) -> Result<String, RpcError> {
        let ledger_proof = self.ledger().get_ledger_inclusion_proof(record_commitment)?;
        Ok(hex::encode(ledger_proof.to_bytes_le().expect("Failed to serialize ledger proof")))
    }

    /// Returns transactions in the node's memory pool.
    async fn get_memory_pool(&self) -> Result<Vec<Transaction<N>>, RpcError> {
        Ok(self.state.prover().memory_pool().read().await.transactions())
    }

    /// Returns a transaction with metadata and decrypted records given the transaction ID.
    async fn get_transaction(&self, transaction_id: N::TransactionID) -> Result<Value, RpcError> {
        let transaction: Transaction<N> = self.ledger().get_transaction(&transaction_id)?;
        let metadata = self.ledger().get_transaction_metadata(&transaction_id)?;
        let decrypted_records: Vec<Record<N>> = transaction.to_records().collect();
        Ok(serde_json::json!({ "transaction": transaction, "metadata": metadata, "decrypted_records": decrypted_records }))
    }

    /// Returns a transition given the transition ID.
    async fn get_transition(&self, transition_id: N::TransitionID) -> Result<Transition<N>, RpcError> {
        Ok(self.ledger().get_transition(&transition_id)?)
    }

    /// Returns the peers currently connected to this node.
    async fn get_connected_peers(&self) -> Result<Vec<SocketAddr>, RpcError> {
        Ok(self.state.peers().connected_peers().await)
    }

    /// Returns the current state of this node.
    async fn get_node_state(&self) -> Result<Value, RpcError> {
        let candidate_peers = self.state.peers().candidate_peers().await;
        let connected_peers = self.state.peers().connected_peers().await;
        let number_of_candidate_peers = candidate_peers.len();
        let number_of_connected_peers = connected_peers.len();
        let number_of_connected_sync_nodes = self.state.peers().number_of_connected_sync_nodes().await;

        let latest_block_hash = self.ledger().latest_block_hash();
        let latest_block_height = self.ledger().latest_block_height();
        let latest_cumulative_weight = self.ledger().latest_cumulative_weight();

        Ok(serde_json::json!({
            "address": self.address,
            "candidate_peers": candidate_peers,
            "connected_peers": connected_peers,
            "latest_block_hash": latest_block_hash,
            "latest_block_height": latest_block_height,
            "latest_cumulative_weight": latest_cumulative_weight,
            "launched": format!("{} minutes ago", self.launched.elapsed().as_secs() / 60),
            "number_of_candidate_peers": number_of_candidate_peers,
            "number_of_connected_peers": number_of_connected_peers,
            "number_of_connected_sync_nodes": number_of_connected_sync_nodes,
            "software": format!("snarkOS {}", env!("CARGO_PKG_VERSION")),
            "status": E::status().to_string(),
            "type": E::NODE_TYPE,
            "version": E::MESSAGE_VERSION,
        }))
    }

    /// Returns the transaction ID. If the given transaction is valid, it is added to the memory pool and propagated to all peers.
    async fn send_transaction(&self, transaction_hex: String) -> Result<N::TransactionID, RpcError> {
        let transaction: Transaction<N> = FromBytes::from_bytes_le(&hex::decode(transaction_hex)?)?;
        // Route an `UnconfirmedTransaction` to the prover.
        let request = ProverRequest::UnconfirmedTransaction("0.0.0.0:3032".parse().unwrap(), transaction.clone());
        if let Err(error) = self.state.prover().router().send(request).await {
            warn!("[UnconfirmedTransaction] {}", error);
        }
        Ok(transaction.transaction_id())
    }

    /// Returns the amount of shares submitted by a given prover.
    async fn get_shares_for_prover(&self, prover: Address<N>) -> Result<u64, RpcError> {
        Ok(self.state.operator().get_shares_for_prover(&prover))
    }

    /// Returns the amount of shares submitted to the operator in total.
    async fn get_shares(&self) -> u64 {
        let shares = self.state.operator().to_shares();
        shares.iter().map(|(_, share)| share.values().sum::<u64>()).sum()
    }

    /// Returns a list of all provers that have submitted shares to the operator.
    async fn get_provers(&self) -> Value {
        let provers = self.state.operator().get_provers();
        serde_json::json!(provers)
    }
}
