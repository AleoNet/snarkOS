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
    network::{
        rpc::{rpc::*, rpc_trait::RpcFunctions},
        Ledger,
    },
    Environment,
    LedgerRequest,
    LedgerRouter,
};
use snarkvm::{
    dpc::{Block, Network, RecordCiphertext, Transaction, Transition},
    utilities::FromBytes,
};

use anyhow::anyhow;
use chrono::Utc;
use jsonrpc_core::{IoDelegate, MetaIoHandler, Params, Value};
use serde::{de::DeserializeOwned, Serialize};
use snarkvm::utilities::ToBytes;
use std::{
    cmp::{max, min},
    future::Future,
    ops::Deref,
    sync::Arc,
};
use tokio::sync::RwLock;

type JsonRPCError = jsonrpc_core::Error;

pub const MAX_RESPONSE_BLOCKS: u32 = 50;

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
}

impl From<std::io::Error> for RpcError {
    fn from(error: std::io::Error) -> Self {
        RpcError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<RpcError> for std::io::Error {
    fn from(error: RpcError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
    }
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

#[doc(hidden)]
pub struct RpcInner<N: Network, E: Environment> {
    ledger: Arc<RwLock<Ledger<N>>>,
    ledger_router: LedgerRouter<N, E>,
    /// RPC credentials for accessing guarded endpoints
    pub(crate) credentials: Option<RpcCredentials>,
}

impl<N: Network, E: Environment> RpcImpl<N, E> {
    /// Creates a new struct for calling public and private RPC endpoints.
    pub fn new(credentials: Option<RpcCredentials>, ledger: Arc<RwLock<Ledger<N>>>, ledger_router: LedgerRouter<N, E>) -> Self {
        Self(Arc::new(RpcInner {
            ledger,
            credentials,
            ledger_router,
        }))
    }

    /// A helper function used to pass a single value to the RPC handler.
    pub async fn map_rpc_singlet<A: DeserializeOwned, O: Serialize, Fut: Future<Output = Result<O, RpcError>>, F: Fn(Self, A) -> Fut>(
        self,
        callee: F,
        params: Params,
        _meta: Meta,
    ) -> Result<Value, JsonRPCError> {
        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };
        if value.len() != 1 {
            return Err(JsonRPCError::invalid_params("Invalid params length".to_string()));
        }
        let val: A =
            serde_json::from_value(value[0].clone()).map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match callee(self, val).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// A helper function used to pass two values to the RPC handler.
    pub async fn map_rpc_doublet<A: DeserializeOwned, O: Serialize, Fut: Future<Output = Result<O, RpcError>>, F: Fn(Self, A, A) -> Fut>(
        self,
        callee: F,
        params: Params,
        _meta: Meta,
    ) -> Result<Value, JsonRPCError> {
        let value = match params {
            Params::Array(arr) => arr,
            _ => return Err(JsonRPCError::invalid_request()),
        };
        if value.len() != 2 {
            return Err(JsonRPCError::invalid_params("Invalid params length".to_string()));
        }
        let first_val: A =
            serde_json::from_value(value[0].clone()).map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;
        let second_val: A =
            serde_json::from_value(value[1].clone()).map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match callee(self, first_val, second_val).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// A helper function used to pass calls to the RPC handler.
    pub async fn map_rpc<O: Serialize, Fut: Future<Output = Result<O, RpcError>>, F: Fn(Self) -> Fut>(
        self,
        callee: F,
        params: Params,
        _meta: Meta,
    ) -> Result<Value, JsonRPCError> {
        params.expect_no_params()?;

        match callee(self).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

    /// Expose the public functions as RPC enpoints
    pub fn add(&self, io: &mut MetaIoHandler<Meta>) {
        let mut d = IoDelegate::<Self, Meta>::new(Arc::new(self.clone()));

        d.add_method_with_meta("latestblock", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.latest_block().await }, params, meta)
        });
        d.add_method_with_meta("latestblockheight", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.latest_block_height().await }, params, meta)
        });
        d.add_method_with_meta("latestblockhash", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.latest_block_hash().await }, params, meta)
        });
        d.add_method_with_meta("getblock", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_block(x).await }, params, meta)
        });
        d.add_method_with_meta("getblocks", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_doublet(|rpc, x, y| async move { rpc.get_blocks(x, y).await }, params, meta)
        });
        d.add_method_with_meta("getblockheight", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_block_height(x).await }, params, meta)
        });
        d.add_method_with_meta("getblockhash", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_block_hash(x).await }, params, meta)
        });
        d.add_method_with_meta("gettransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_transaction(x).await }, params, meta)
        });
        d.add_method_with_meta("gettransition", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_transition(x).await }, params, meta)
        });
        d.add_method_with_meta("getciphertext", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_ciphertext(x).await }, params, meta)
        });
        d.add_method_with_meta("sendtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.send_transaction(x).await }, params, meta)
        });
        d.add_method_with_meta("ledgerproof", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.ledger_proof(x).await }, params, meta)
        });
        // d.add_method_with_meta("validaterawtransaction", |rpc, params, meta| {
        //     let rpc = rpc.clone();
        //     rpc.map_rpc_singlet(
        //         |rpc, x| async move { rpc.validate_raw_transaction(x).await },
        //         params,
        //         meta,
        //     )
        // });
        // d.add_method_with_meta("getblocktemplate", |rpc, params, meta| {
        //     let rpc = rpc.clone();
        //     rpc.map_rpc(|rpc| async move { rpc.get_block_template().await }, params, meta)
        // });

        io.extend_with(d)
    }
}

#[async_trait::async_trait]
impl<N: Network, E: Environment> RpcFunctions<N> for RpcImpl<N, E> {
    /// Returns the block from the head of the canonical chain.
    async fn latest_block(&self) -> Result<Block<N>, RpcError> {
        Ok(self.ledger.read().await.latest_block()?)
    }

    /// Returns the number of blocks in the canonical chain.
    async fn latest_block_height(&self) -> Result<u32, RpcError> {
        Ok(self.ledger.read().await.latest_block_height())
    }

    /// Returns the block hash from the head of the canonical chain.
    async fn latest_block_hash(&self) -> Result<N::BlockHash, RpcError> {
        Ok(self.ledger.read().await.latest_block_hash())
    }

    /// Returns the block given the block height.
    async fn get_block(&self, block_height: u32) -> Result<Block<N>, RpcError> {
        Ok(self.ledger.read().await.get_block(block_height)?)
    }

    /// Returns up to `MAX_RESPONSE_BLOCKS` blocks from the given `start_block_height` to `end_block_height`.
    async fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>, RpcError> {
        if end_block_height < start_block_height {
            return Err(anyhow!("Invalid start/end block heights: {}, {}", start_block_height, end_block_height).into());
        }
        // Ensure we don't get more than MAX_RESPONSE_BLOCKS blocks
        let safe_start_height = max(start_block_height, end_block_height.saturating_sub(MAX_RESPONSE_BLOCKS - 1));
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

    /// Returns a transaction given the transaction ID.
    async fn get_transaction(&self, transaction_id: serde_json::Value) -> Result<Transaction<N>, RpcError> {
        let transaction_id: N::TransactionID = serde_json::from_value(transaction_id)?;
        Ok(self.ledger.read().await.get_transaction(&transaction_id)?)
    }

    /// Returns a transition given the transition ID.
    async fn get_transition(&self, transition_id: serde_json::Value) -> Result<Transition<N>, RpcError> {
        let transition_id: N::TransitionID = serde_json::from_value(transition_id)?;
        Ok(self.ledger.read().await.get_transition(&transition_id)?)
    }

    /// Returns the ciphertext given the ciphertext ID.
    async fn get_ciphertext(&self, ciphertext_id: serde_json::Value) -> Result<RecordCiphertext<N>, RpcError> {
        let ciphertext_id: N::CiphertextID = serde_json::from_value(ciphertext_id)?;
        Ok(self.ledger.read().await.get_ciphertext(&ciphertext_id)?)
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

    /// Returns the ledger proof for a given record commitment.
    async fn ledger_proof(&self, record_commitment: serde_json::Value) -> Result<String, RpcError> {
        let record_commitment: N::Commitment = serde_json::from_value(record_commitment)?;
        let ledger_proof = self.ledger.read().await.get_ledger_inclusion_proof(&record_commitment)?;
        Ok(hex::encode(ledger_proof.to_bytes_le().expect("Failed to serialize ledger proof")))
    }

    // /// Validate and return if the transaction is valid.
    // async fn validate_raw_transaction(&self, transaction_bytes: String) -> Result<bool, RpcError> {
    //     let transaction_bytes = hex::decode(transaction_bytes)?;
    //     let transaction = Testnet1Transaction::read_le(&transaction_bytes[..])?;
    //
    //     Ok(self
    //         .sync_handler()?
    //         .consensus
    //         .verify_transactions(vec![transaction.serialize()?])
    //         .await)
    // }
    //
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
