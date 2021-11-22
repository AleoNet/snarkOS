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
    LedgerRequest,
    LedgerRouter,
    Peers,
    Wallet,
};
use snarkos_ledger::Metadata;
use snarkvm::{
    dpc::{Address, Block, BlockHeader, Network, RecordCiphertext, Transaction, Transactions, Transition},
    utilities::FromBytes,
};

use jsonrpc_core::Value;
use snarkvm::utilities::ToBytes;
use std::{cmp::max, collections::HashMap, fs, net::SocketAddr, ops::Deref, path::PathBuf, str::FromStr, sync::Arc};
use tokio::sync::RwLock;

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
    peers: Arc<RwLock<Peers<N, E>>>,
    ledger: LedgerReader<N>,
    ledger_router: LedgerRouter<N, E>,
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
        peers: Arc<RwLock<Peers<N, E>>>,
        ledger: LedgerReader<N>,
        ledger_router: LedgerRouter<N, E>,
    ) -> Self {
        Self(Arc::new(RpcInner {
            status,
            peers,
            ledger,
            ledger_router,
            credentials,
        }))
    }
}

#[async_trait::async_trait]
impl<N: Network, E: Environment> RpcFunctions<N> for RpcImpl<N, E> {
    /// Returns the latest block from the canonical chain.
    async fn latest_block(&self) -> Result<Block<N>, RpcError> {
        Ok(self.ledger.read().await.latest_block())
    }

    /// Returns the latest block height from the canonical chain.
    async fn latest_block_height(&self) -> Result<u32, RpcError> {
        Ok(self.ledger.read().await.latest_block_height())
    }

    /// Returns the latest block hash from the canonical chain.
    async fn latest_block_hash(&self) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger.read().await.latest_block_hash())
    }

    /// Returns the latest block header from the canonical chain.
    async fn latest_block_header(&self) -> Result<BlockHeader<N>, RpcError> {
        Ok(self.ledger.read().await.latest_block_header())
    }

    /// Returns the latest block transactions from the canonical chain.
    async fn latest_block_transactions(&self) -> Result<Transactions<N>, RpcError> {
        Ok(self.ledger.read().await.latest_block_transactions())
    }

    /// Returns the latest ledger root from the canonical chain.
    async fn latest_ledger_root(&self) -> Result<N::LedgerRoot, RpcError> {
        Ok(self.ledger.read().await.latest_ledger_root())
    }

    /// Returns the block given the block height.
    async fn get_block(&self, block_height: u32) -> Result<Block<N>, RpcError> {
        Ok(self.ledger.read().await.get_block(block_height)?)
    }

    /// Returns up to `MAXIMUM_BLOCK_REQUEST` blocks from the given `start_block_height` to `end_block_height` (inclusive).
    async fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>, RpcError> {
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        Ok(self.ledger.read().await.get_blocks(safe_start_height, end_block_height)?)
    }

    /// Returns the block height for the given the block hash.
    async fn get_block_height(&self, block_hash: serde_json::Value) -> Result<u32, RpcError> {
        let block_hash: N::BlockHash = serde_json::from_value(block_hash)?;
        Ok(self.ledger.read().await.get_block_height(&block_hash)?)
    }

    /// Returns the block hash for the given block height, if it exists in the canonical chain.
    async fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger.read().await.get_block_hash(block_height)?)
    }

    /// Returns up to `MAXIMUM_BLOCK_REQUEST` block hashes from the given `start_block_height` to `end_block_height` (inclusive).
    async fn get_block_hashes(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<N::BlockHash>, RpcError> {
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(E::MAXIMUM_BLOCK_REQUEST - 1));
        Ok(self.ledger.read().await.get_block_hashes(safe_start_height, end_block_height)?)
    }

    /// Returns the block header for the given the block height.
    async fn get_block_header(&self, block_height: u32) -> Result<BlockHeader<N>, RpcError> {
        Ok(self.ledger.read().await.get_block_header(block_height)?)
    }

    async fn get_blocks_mined(&self) -> Result<Value, RpcError> {
        let mut records = HashMap::new();
        let mut num_records = 0usize;

        let data_path = ".aleo";
        let dir = fs::read_dir(PathBuf::from(data_path))?;
        for entry in dir {
            let entry = entry?;
            let path = entry.path();
            let name = entry
                .file_name()
                .into_string()
                .expect("Should be able to convert OsString into String");
            if path.is_dir() && Address::<N>::from_str(&name).is_ok() {
                let wallet = Wallet::<N>::new(&name, &data_path, true)?;
                let records_for_address = wallet.records()?;
                num_records += records_for_address.len();
                records.insert(name, records_for_address);
            }
        }

        let read_ledger = self.ledger.read().await;
        let canon_records: Vec<&Transaction<N>> = records
            .iter()
            .filter(|(_, rs)| {
                let canon: Vec<&Transaction<N>> = rs
                    .iter()
                    .filter(|r| {
                        read_ledger
                            .contains_transaction(&r.transaction_id())
                            .expect("Should be able to check if commitment exists")
                    })
                    .collect();
                !canon.is_empty()
            })
            .map(|(_, rs)| rs)
            .flatten()
            .collect();
        Ok(serde_json::json!({"canon_blocks_mined": canon_records.len(), "total_blocks_mined": num_records}))
    }

    /// Returns the transactions from the block of the given block height.
    async fn get_block_transactions(&self, block_height: u32) -> Result<Transactions<N>, RpcError> {
        Ok(self.ledger.read().await.get_block_transactions(block_height)?)
    }

    /// Returns the ciphertext given the ciphertext ID.
    async fn get_ciphertext(&self, ciphertext_id: serde_json::Value) -> Result<RecordCiphertext<N>, RpcError> {
        let ciphertext_id: N::CiphertextID = serde_json::from_value(ciphertext_id)?;
        Ok(self.ledger.read().await.get_ciphertext(&ciphertext_id)?)
    }

    /// Returns the ledger proof for a given record commitment.
    async fn get_ledger_proof(&self, record_commitment: serde_json::Value) -> Result<String, RpcError> {
        let record_commitment: N::Commitment = serde_json::from_value(record_commitment)?;
        let ledger_proof = self.ledger.read().await.get_ledger_inclusion_proof(record_commitment)?;
        Ok(hex::encode(ledger_proof.to_bytes_le().expect("Failed to serialize ledger proof")))
    }

    /// Returns a transaction with metadata given the transaction ID.
    async fn get_transaction(&self, transaction_id: serde_json::Value) -> Result<Value, RpcError> {
        let transaction_id: N::TransactionID = serde_json::from_value(transaction_id)?;
        let transaction: Transaction<N> = self.ledger.read().await.get_transaction(&transaction_id)?;
        let metadata: Metadata<N> = self.ledger.read().await.get_transaction_metadata(&transaction_id)?;
        Ok(serde_json::json!({ "transaction": transaction, "metadata": metadata }))
    }

    /// Returns a transition given the transition ID.
    async fn get_transition(&self, transition_id: serde_json::Value) -> Result<Transition<N>, RpcError> {
        let transition_id: N::TransitionID = serde_json::from_value(transition_id)?;
        Ok(self.ledger.read().await.get_transition(&transition_id)?)
    }

    /// Returns the peers currently connected to this node.
    async fn get_connected_peers(&self) -> Result<Vec<SocketAddr>, RpcError> {
        Ok(self.peers.read().await.connected_peers())
    }

    /// Returns the current state of this node.
    async fn get_node_state(&self) -> Result<Value, RpcError> {
        Ok(serde_json::json!({
            "candidate_peers": self.peers.read().await.candidate_peers(),
            "connected_peers": self.peers.read().await.connected_peers(),
            "latest_block_height": self.ledger.read().await.latest_block_height(),
            "number_of_candidate_peers": self.peers.read().await.number_of_candidate_peers(),
            "number_of_connected_peers": self.peers.read().await.number_of_connected_peers(),
            "status": self.status.to_string(),
            "type": E::NODE_TYPE,
            "version": E::MESSAGE_VERSION,
        }))
    }

    /// Returns the transaction ID. If the given transaction is valid, it is added to the memory pool and propagated to all peers.
    async fn send_transaction(&self, transaction_hex: String) -> Result<N::TransactionID, RpcError> {
        let transaction: Transaction<N> = FromBytes::from_bytes_le(&hex::decode(transaction_hex)?)?;
        // Route an `UnconfirmedTransaction` to the ledger.
        let request = LedgerRequest::UnconfirmedTransaction("0.0.0.0:3032".parse().unwrap(), transaction.clone());
        if let Err(error) = self.ledger_router.send(request).await {
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
