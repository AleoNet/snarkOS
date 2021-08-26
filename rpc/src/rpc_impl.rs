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

use crate::{error::RpcError, rpc_trait::RpcFunctions, rpc_types::*};
use futures::Future;
use jsonrpc_core::{IoDelegate, MetaIoHandler, Params, Value};
use serde::{de::DeserializeOwned, Serialize};
use snarkos_consensus::{get_block_reward, ConsensusParameters};
use snarkos_metrics::{snapshots::NodeStats, stats::NODE_STATS};
use snarkos_network::{KnownNetwork, NetworkMetrics, Node, Sync};
use snarkos_storage::{BlockStatus, Digest, DynStorage, VMTransaction};
use snarkvm_dpc::{
    testnet1::instantiated::{Testnet1DPC, Testnet1Transaction},
    TransactionScheme,
};
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes_le,
    CanonicalSerialize,
};

use chrono::Utc;

use std::{ops::Deref, sync::Arc};

type JsonRPCError = jsonrpc_core::Error;

/// Implements JSON-RPC HTTP endpoint functions for a node.
/// The constructor is given Arc::clone() copies of all needed node components.
#[derive(Clone)]
pub struct RpcImpl(Arc<RpcInner>);

impl Deref for RpcImpl {
    type Target = RpcInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct RpcInner {
    /// Blockchain database storage.
    pub(crate) storage: DynStorage,

    /// RPC credentials for accessing guarded endpoints
    pub(crate) credentials: Option<RpcCredentials>,

    /// A clone of the network Node
    pub(crate) node: Node,
}

impl RpcImpl {
    /// Creates a new struct for calling public and private RPC endpoints.
    pub fn new(storage: DynStorage, credentials: Option<RpcCredentials>, node: Node) -> Self {
        Self(Arc::new(RpcInner {
            storage,
            credentials,
            node,
        }))
    }

    pub fn sync_handler(&self) -> Result<&Arc<Sync>, RpcError> {
        self.node.sync().ok_or(RpcError::NoConsensus)
    }

    pub fn consensus_parameters(&self) -> Result<&ConsensusParameters, RpcError> {
        Ok(&self.sync_handler()?.consensus.parameters)
    }

    pub fn dpc(&self) -> Result<&Testnet1DPC, RpcError> {
        Ok(&self.sync_handler()?.consensus.dpc)
    }

    pub fn known_network(&self) -> Result<&KnownNetwork, RpcError> {
        self.node.known_network().ok_or(RpcError::NoKnownNetwork)
    }

    pub async fn map_rpc_singlet<
        A: DeserializeOwned,
        O: Serialize,
        Fut: Future<Output = Result<O, RpcError>>,
        F: Fn(Self, A) -> Fut,
    >(
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

        let val: A = serde_json::from_value(value[0].clone())
            .map_err(|e| JsonRPCError::invalid_params(format!("Invalid params: {}.", e)))?;

        match callee(self, val).await {
            Ok(result) => Ok(serde_json::to_value(result).expect("serialization failed")),
            Err(err) => Err(JsonRPCError::invalid_params(err.to_string())),
        }
    }

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

        d.add_method_with_meta("getblock", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_block(x).await }, params, meta)
        });
        d.add_method_with_meta("getblockcount", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_block_count().await }, params, meta)
        });
        d.add_method_with_meta("getbestblockhash", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_best_block_hash().await }, params, meta)
        });
        d.add_method_with_meta("getblockhash", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_block_hash(x).await }, params, meta)
        });
        d.add_method_with_meta("getrawtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_raw_transaction(x).await }, params, meta)
        });
        d.add_method_with_meta("gettransactioninfo", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.get_transaction_info(x).await }, params, meta)
        });
        d.add_method_with_meta("decoderawtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(
                |rpc, x| async move { rpc.decode_raw_transaction(x).await },
                params,
                meta,
            )
        });
        d.add_method_with_meta("sendtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(|rpc, x| async move { rpc.send_raw_transaction(x).await }, params, meta)
        });
        d.add_method_with_meta("validaterawtransaction", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc_singlet(
                |rpc, x| async move { rpc.validate_raw_transaction(x).await },
                params,
                meta,
            )
        });
        d.add_method_with_meta("getconnectioncount", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_connection_count().await }, params, meta)
        });
        d.add_method_with_meta("getpeerinfo", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_peer_info().await }, params, meta)
        });
        d.add_method_with_meta("getnodeinfo", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_node_info().await }, params, meta)
        });
        d.add_method_with_meta("getnodestats", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_node_stats().await }, params, meta)
        });
        d.add_method_with_meta("getblocktemplate", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_block_template().await }, params, meta)
        });
        d.add_method_with_meta("getnetworkgraph", |rpc, params, meta| {
            let rpc = rpc.clone();
            rpc.map_rpc(|rpc| async move { rpc.get_network_graph().await }, params, meta)
        });

        io.extend_with(d)
    }
}

#[async_trait::async_trait]
impl RpcFunctions for RpcImpl {
    /// Returns information about a block from a block hash.
    async fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError> {
        let block_hash = hex::decode(&block_hash_string)?;
        if block_hash.len() != 32 {
            return Err(RpcError::InvalidBlockHash(block_hash_string));
        }

        let block_header_hash: Digest = block_hash[..].into();
        let height = match self.storage.get_block_state(&block_header_hash).await? {
            BlockStatus::Committed(block_num) => Some(block_num),
            BlockStatus::Uncommitted => None,
            BlockStatus::Unknown => return Err(RpcError::InvalidBlockHash(block_hash_string)),
        };

        let canon = self.storage.canon().await?;

        let confirmations = match height {
            Some(block_height) => canon.block_height - block_height,
            None => 0,
        };

        let block = self.storage.get_block(&block_header_hash).await?;
        let mut transactions = Vec::with_capacity(block.transactions.len());

        for transaction in block.transactions.iter() {
            transactions.push(hex::encode(&transaction.id));
        }

        Ok(BlockInfo {
            hash: block_hash_string,
            height: height.map(|x| x as u32),
            confirmations: confirmations as u32,
            size: block.serialize().len(), // todo: we should not do this
            previous_block_hash: block.header.previous_block_hash.to_string(),
            merkle_root: block.header.merkle_root_hash.to_string(),
            pedersen_merkle_root_hash: block.header.pedersen_merkle_root_hash.to_string(),
            proof: block.header.proof.to_string(),
            time: block.header.time,
            difficulty_target: block.header.difficulty_target,
            nonce: block.header.nonce,
            transactions,
        })
    }

    /// Returns the number of blocks in the canonical chain, including the genesis.
    async fn get_block_count(&self) -> Result<u32, RpcError> {
        let canon = self.storage.canon().await?;
        Ok(canon.block_height as u32 + 1)
    }

    /// Returns the block hash of the head of the canonical chain.
    async fn get_best_block_hash(&self) -> Result<String, RpcError> {
        let canon = self.storage.canon().await?;

        Ok(hex::encode(&canon.hash.0))
    }

    /// Returns the block hash of the index specified if it exists in the canonical chain.
    async fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError> {
        let block_hash = self.storage.get_block_hash(block_height).await?;

        Ok(block_hash
            .map(|x| hex::encode(&x))
            .unwrap_or_else(|| "invalid block number".to_string()))
    }

    /// Returns the hex encoded bytes of a transaction from its transaction id.
    async fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError> {
        let transaction_id = hex::decode(transaction_id)?;
        let transaction = self.storage.get_transaction(transaction_id[..].into()).await?;

        Ok(hex::encode(&to_bytes_le![&transaction]?))
    }

    /// Returns information about a transaction from a transaction id.
    async fn get_transaction_info(&self, transaction_id: String) -> Result<TransactionInfo, RpcError> {
        let transaction_bytes = self.get_raw_transaction(transaction_id).await?;
        self.decode_raw_transaction(transaction_bytes).await
    }

    /// Returns information about a transaction from serialized transaction bytes.
    async fn decode_raw_transaction(&self, transaction_bytes: String) -> Result<TransactionInfo, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Testnet1Transaction::read_le(&transaction_bytes[..])?;

        let mut old_serial_numbers = Vec::with_capacity(transaction.old_serial_numbers().len());

        for sn in transaction.old_serial_numbers() {
            let mut serial_number: Vec<u8> = vec![];
            CanonicalSerialize::serialize(sn, &mut serial_number).unwrap();
            old_serial_numbers.push(hex::encode(serial_number));
        }

        let mut new_commitments = Vec::with_capacity(transaction.new_commitments().len());

        for cm in transaction.new_commitments() {
            new_commitments.push(hex::encode(to_bytes_le![cm]?));
        }

        let memo = hex::encode(to_bytes_le![transaction.memorandum()]?);

        let mut signatures = Vec::with_capacity(transaction.signatures.len());
        for sig in &transaction.signatures {
            signatures.push(hex::encode(to_bytes_le![sig]?));
        }

        let mut encrypted_records = Vec::with_capacity(transaction.encrypted_records.len());

        for encrypted_record in &transaction.encrypted_records {
            encrypted_records.push(hex::encode(to_bytes_le![encrypted_record]?));
        }

        let transaction_id = transaction.transaction_id()?;

        let block_number = match self.storage.get_transaction_location(transaction_id.into()).await? {
            Some(block_location) => match self.storage.get_block_state(&block_location.block_hash).await? {
                BlockStatus::Committed(n) => Some(n as u32),
                _ => None,
            },
            None => None,
        };

        let transaction_metadata = TransactionMetadata { block_number };

        Ok(TransactionInfo {
            txid: hex::encode(&transaction_id),
            size: transaction_bytes.len(),
            old_serial_numbers,
            new_commitments,
            memo,
            network_id: transaction.network.id(),
            digest: hex::encode(to_bytes_le![transaction.ledger_digest]?),
            transaction_proof: hex::encode(to_bytes_le![transaction.transaction_proof]?),
            program_commitment: hex::encode(to_bytes_le![transaction.program_commitment]?),
            local_data_root: hex::encode(to_bytes_le![transaction.local_data_root]?),
            value_balance: transaction.value_balance.0,
            signatures,
            encrypted_records,
            transaction_metadata,
        })
    }

    /// Send raw transaction bytes to this node to be added into the mempool.
    /// If valid, the transaction will be stored and propagated to all peers.
    /// Returns the transaction id if valid.
    async fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Testnet1Transaction::read_le(&transaction_bytes[..])?;
        let transaction_hex_id = hex::encode(transaction.transaction_id()?);

        if !self
            .sync_handler()?
            .consensus
            .receive_transaction(transaction.serialize()?)
            .await
        {
            return Ok("Transaction did not verify".into());
        }

        Ok(transaction_hex_id)
    }

    /// Validate and return if the transaction is valid.
    async fn validate_raw_transaction(&self, transaction_bytes: String) -> Result<bool, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Testnet1Transaction::read_le(&transaction_bytes[..])?;

        Ok(self
            .sync_handler()?
            .consensus
            .verify_transactions(vec![transaction.serialize()?])
            .await)
    }

    /// Fetch the number of connected peers this node has.
    async fn get_connection_count(&self) -> Result<usize, RpcError> {
        // Create a temporary tokio runtime to make an asynchronous function call
        let number = self.node.peer_book.get_active_peer_count();

        Ok(number as usize)
    }

    /// Returns this nodes connected peers.
    async fn get_peer_info(&self) -> Result<PeerInfo, RpcError> {
        // Create a temporary tokio runtime to make an asynchronous function call
        let peers = self.node.peer_book.connected_peers();

        Ok(PeerInfo { peers })
    }

    /// Returns data about the node.
    async fn get_node_info(&self) -> Result<NodeInfo, RpcError> {
        Ok(NodeInfo {
            listening_addr: self.node.config.desired_address,
            is_bootnode: self.node.config.is_bootnode(),
            is_miner: self.sync_handler()?.is_miner,
            is_syncing: self.node.is_syncing_blocks(),
            launched: self.node.launched,
            version: env!("CARGO_PKG_VERSION").into(),
            min_peers: self.node.config.minimum_number_of_connected_peers(),
            max_peers: self.node.config.maximum_number_of_connected_peers(),
        })
    }

    /// Returns statistics related to the node.
    async fn get_node_stats(&self) -> Result<NodeStats, RpcError> {
        let metrics = NODE_STATS.snapshot();

        Ok(metrics)
    }

    /// Returns the current mempool and sync information known by this node.
    async fn get_block_template(&self) -> Result<BlockTemplate, RpcError> {
        let canon = self.storage.canon().await?;

        let block = self.storage.get_block_header(&canon.hash).await?;

        let time = Utc::now().timestamp();

        let full_transactions = self.node.expect_sync().consensus.fetch_memory_pool().await;

        let transaction_strings = full_transactions
            .iter()
            .map(|x| Ok(hex::encode(to_bytes_le![x]?)))
            .collect::<Result<Vec<_>, RpcError>>()?;

        let mut coinbase_value = get_block_reward(canon.block_height as u32 + 1);
        for transaction in full_transactions.iter() {
            coinbase_value = coinbase_value.add(transaction.value_balance)
        }

        Ok(BlockTemplate {
            previous_block_hash: hex::encode(&block.hash().0),
            block_height: canon.block_height as u32 + 1,
            time,
            difficulty_target: self.consensus_parameters()?.get_block_difficulty(&block, time),
            transactions: transaction_strings,
            coinbase_value: coinbase_value.0 as u64,
        })
    }

    async fn get_network_graph(&self) -> Result<NetworkGraph, RpcError> {
        // Copy the connections as the data must not change throughout the metrics computation.
        let known_network = self.known_network()?;
        let connections = known_network.connections();

        // Collect the edges.
        let edges = connections
            .iter()
            .map(|connection| Edge {
                source: connection.source,
                target: connection.target,
            })
            .collect();

        // Compute the metrics or provide default values if there are no known connections yet.
        let network_metrics = if !connections.is_empty() {
            NetworkMetrics::new(connections)
        } else {
            NetworkMetrics::default()
        };

        // Collect the vertices with the metrics.
        let vertices: Vec<Vertice> = network_metrics
            .centrality
            .iter()
            .map(|(addr, node_centrality)| Vertice {
                addr: *addr,
                is_bootnode: self.node.config.bootnodes().contains(addr),
                degree_centrality: node_centrality.degree_centrality,
                eigenvector_centrality: node_centrality.eigenvector_centrality,
                fiedler_value: node_centrality.fiedler_value,
            })
            .collect();

        let potential_forks = known_network
            .potential_forks()
            .into_iter()
            .map(|(height, members)| PotentialFork { height, members })
            .collect();

        let node_count = if network_metrics.node_count == 0 {
            known_network.nodes().len()
        } else {
            network_metrics.node_count
        };

        Ok(NetworkGraph {
            node_count,
            connection_count: network_metrics.connection_count,
            density: network_metrics.density,
            algebraic_connectivity: network_metrics.algebraic_connectivity,
            degree_centrality_delta: network_metrics.degree_centrality_delta,
            potential_forks,
            vertices,
            edges,
        })
    }
}
