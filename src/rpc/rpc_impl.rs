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

//! Implementation of public RPC endpoints.
//!
//! See [RpcFunctions](../trait.RpcFunctions.html) for documentation of public endpoints.

use crate::{
    helpers::Status,
    rpc::{rpc::*, rpc_trait::RpcFunctions},
    Environment,
    LedgerReader,
    Peers,
    ProverRequest,
    ProverRouter,
};
use snarkos_storage::Metadata;
use snarkvm::{
    dpc::{Block, BlockHeader, Network, Transaction, Transactions, Transition},
    utilities::FromBytes,
};

use jsonrpc_core::Value;
use snarkvm::utilities::ToBytes;
use std::{cmp::max, net::SocketAddr, ops::Deref, sync::Arc};

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("{}", _0)]
    AnyhowError(#[from] anyhow::Error),
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
    #[error("{}", _0)]
    FromHexError(#[from] hex::FromHexError),
    #[error("{}", _0)]
    Message(String),
    #[error("{}", _0)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("{}", _0)]
    SerdeJson(#[from] serde_json::Error),
    #[error("{}", _0)]
    StdIOError(#[from] std::io::Error),
}

impl From<RpcError> for std::io::Error {
    fn from(error: RpcError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
    }
}

#[doc(hidden)]
pub struct RpcInner<N: Network, E: Environment> {
    status: Status,
    peers: Arc<Peers<N, E>>,
    ledger: LedgerReader<N>,
    prover_router: ProverRouter<N>,
    /// RPC credentials for accessing guarded endpoints
    #[allow(unused)]
    pub(crate) credentials: RpcCredentials,
}

/// Implements RPC HTTP endpoint functions for a node.
#[derive(Clone)]
pub struct RpcImpl<N: Network, E: Environment>(Arc<RpcInner<N, E>>);

impl<N: Network, E: Environment> Deref for RpcImpl<N, E> {
    type Target = RpcInner<N, E>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<N: Network, E: Environment> RpcImpl<N, E> {
    /// Creates a new struct for calling public and private RPC endpoints.
    pub fn new(
        credentials: RpcCredentials,
        status: Status,
        peers: Arc<Peers<N, E>>,
        ledger: LedgerReader<N>,
        prover_router: ProverRouter<N>,
    ) -> Self {
        Self(Arc::new(RpcInner {
            status,
            peers,
            ledger,
            prover_router,
            credentials,
        }))
    }
}

#[async_trait::async_trait]
impl<N: Network, E: Environment> RpcFunctions<N> for RpcImpl<N, E> {
    /// Returns the latest block from the canonical chain.
    async fn latest_block(&self) -> Result<Block<N>, RpcError> {
        Ok(self.ledger.latest_block())
    }

    /// Returns the latest block height from the canonical chain.
    async fn latest_block_height(&self) -> Result<u32, RpcError> {
        Ok(self.ledger.latest_block_height())
    }

    /// Returns the latest cumulative weight from the canonical chain.
    async fn latest_cumulative_weight(&self) -> Result<u128, RpcError> {
        Ok(self.ledger.latest_cumulative_weight())
    }

    /// Returns the latest block hash from the canonical chain.
    async fn latest_block_hash(&self) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger.latest_block_hash())
    }

    /// Returns the latest block header from the canonical chain.
    async fn latest_block_header(&self) -> Result<BlockHeader<N>, RpcError> {
        Ok(self.ledger.latest_block_header())
    }

    /// Returns the latest block transactions from the canonical chain.
    async fn latest_block_transactions(&self) -> Result<Transactions<N>, RpcError> {
        Ok(self.ledger.latest_block_transactions())
    }

    /// Returns the latest ledger root from the canonical chain.
    async fn latest_ledger_root(&self) -> Result<N::LedgerRoot, RpcError> {
        Ok(self.ledger.latest_ledger_root())
    }

    /// Returns the block given the block height.
    async fn get_block(&self, block_height: u32) -> Result<Block<N>, RpcError> {
        Ok(self.ledger.get_block(block_height)?)
    }

    /// Returns up to `MAXIMUM_BLOCK_REQUEST` blocks from the given `start_block_height` to `end_block_height` (inclusive).
    async fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>, RpcError> {
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        Ok(self.ledger.get_blocks(safe_start_height, end_block_height)?)
    }

    /// Returns the block height for the given the block hash.
    async fn get_block_height(&self, block_hash: serde_json::Value) -> Result<u32, RpcError> {
        let block_hash: N::BlockHash = serde_json::from_value(block_hash)?;
        Ok(self.ledger.get_block_height(&block_hash)?)
    }

    /// Returns the block hash for the given block height, if it exists in the canonical chain.
    async fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger.get_block_hash(block_height)?)
    }

    /// Returns up to `MAXIMUM_BLOCK_REQUEST` block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    async fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>, RpcError> {
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        Ok(self.ledger.get_block_hashes(safe_start_height, end_block_height)?)
    }

    /// Returns the block header for the given the block height.
    async fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>, RpcError> {
        Ok(self.ledger.get_block_header(block_height)?)
    }

    /// Returns the transactions from the block of the given block height.
    async fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>, RpcError> {
        Ok(self.ledger.get_block_transactions(block_height)?)
    }

    /// Returns the ciphertext given the commitment.
    async fn get_ciphertext(&self, commitment: serde_json::Value) -> Result<N::RecordCiphertext, RpcError> {
        let commitment: N::Commitment = serde_json::from_value(commitment)?;
        Ok(self.ledger.get_ciphertext(&commitment)?)
    }

    /// Returns the ledger proof for a given record commitment.
    async fn get_ledger_proof(&self, record_commitment: serde_json::Value) -> Result<String, RpcError> {
        let record_commitment: N::Commitment = serde_json::from_value(record_commitment)?;
        let ledger_proof = self.ledger.get_ledger_inclusion_proof(record_commitment)?;
        Ok(hex::encode(ledger_proof.to_bytes_le().expect("Failed to serialize ledger proof")))
    }

    /// Returns a transaction with metadata given the transaction ID.
    async fn get_transaction(&self, transaction_id: serde_json::Value) -> Result<Value, RpcError> {
        let transaction_id: N::TransactionID = serde_json::from_value(transaction_id)?;
        let transaction: Transaction<N> = self.ledger.get_transaction(&transaction_id)?;
        let metadata: Metadata<N> = self.ledger.get_transaction_metadata(&transaction_id)?;
        Ok(serde_json::json!({ "transaction": transaction, "metadata": metadata }))
    }

    /// Returns a transition given the transition ID.
    async fn get_transition(&self, transition_id: serde_json::Value) -> Result<Transition<N>, RpcError> {
        let transition_id: N::TransitionID = serde_json::from_value(transition_id)?;
        Ok(self.ledger.get_transition(&transition_id)?)
    }

    /// Returns the peers currently connected to this node.
    async fn get_connected_peers(&self) -> Result<Vec<SocketAddr>, RpcError> {
        Ok(self.peers.connected_peers().await)
    }

    /// Returns the current state of this node.
    async fn get_node_state(&self) -> Result<Value, RpcError> {
        let candidate_peers = self.peers.candidate_peers().await;
        let connected_peers = self.peers.connected_peers().await;
        let number_of_candidate_peers = candidate_peers.len();
        let number_of_connected_peers = connected_peers.len();
        let number_of_connected_sync_nodes = self.peers.number_of_connected_sync_nodes().await;

        let latest_block_height = self.ledger.latest_block_height();
        let latest_cumulative_weight = self.ledger.latest_cumulative_weight().to_string();

        Ok(serde_json::json!({
            "candidate_peers": candidate_peers,
            "connected_peers": connected_peers,
            "latest_block_height": latest_block_height,
            "latest_cumulative_weight": latest_cumulative_weight,
            "number_of_candidate_peers": number_of_candidate_peers,
            "number_of_connected_peers": number_of_connected_peers,
            "number_of_connected_sync_nodes": number_of_connected_sync_nodes,
            "software": format!("snarkOS {}", env!("CARGO_PKG_VERSION")),
            "status": self.status.to_string(),
            "type": E::NODE_TYPE,
            "version": E::MESSAGE_VERSION,
        }))
    }

    /// Returns the transaction ID. If the given transaction is valid, it is added to the memory pool and propagated to all peers.
    async fn send_transaction(&self, transaction_hex: String) -> Result<N::TransactionID, RpcError> {
        let transaction: Transaction<N> = FromBytes::from_bytes_le(&hex::decode(transaction_hex)?)?;
        // Route an `UnconfirmedTransaction` to the prover.
        let request = ProverRequest::UnconfirmedTransaction("0.0.0.0:3032".parse().unwrap(), transaction.clone());
        if let Err(error) = self.prover_router.send(request).await {
            warn!("[UnconfirmedTransaction] {}", error);
        }
        Ok(transaction.transaction_id())
    }

    // /// Returns the current mempool and sync information known by this node.
    // async fn get_block_template(&self) -> Result<BlockTemplate, RpcError> {
    //     let canon = self.storage.canon().await?;
    //
    //     let block = self.storage.get_block_header(&canon.hash).await?;
    //
    //     let time = Utc::now().timestamp();
    //
    //     let full_transactions = self.node.expect_sync().consensus.fetch_memory_pool().await;
    //
    //     let transaction_strings = full_transactions
    //         .iter()
    //         .map(|x| Ok(hex::encode(to_bytes_le![x]?)))
    //         .collect::<Result<Vec<_>, RpcError>>()?;
    //
    //     let mut coinbase_value = get_block_reward(canon.block_height as u32 + 1);
    //     for transaction in full_transactions.iter() {
    //         coinbase_value = coinbase_value.add(transaction.value_balance)
    //     }
    //
    //     Ok(BlockTemplate {
    //         previous_block_hash: hex::encode(&block.hash().0),
    //         block_height: canon.block_height as u32 + 1,
    //         time,
    //         difficulty_target: self.consensus_parameters()?.get_block_difficulty(&block, time),
    //         transactions: transaction_strings,
    //         coinbase_value: coinbase_value.0 as u64,
    //     })
    // }
}
