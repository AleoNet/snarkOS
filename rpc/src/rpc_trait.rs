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

//! Definition of the public and private RPC endpoints.

use crate::{error::RpcError, rpc_types::*};
use snarkos_metrics::snapshots::NodeStats;

use std::net::SocketAddr;

/// Definition of public RPC endpoints.
#[async_trait::async_trait]
pub trait RpcFunctions {
    #[doc = include_str!("../documentation/public_endpoints/getblock.md")]
    async fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getblockcount.md")]
    async fn get_block_count(&self) -> Result<u32, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getbestblockhash.md")]
    async fn get_best_block_hash(&self) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getblockhash.md")]
    async fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getrawtransaction.md")]
    async fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/gettransactioninfo.md")]
    async fn get_transaction_info(&self, transaction_id: String) -> Result<TransactionInfo, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/decoderawtransaction.md")]
    async fn decode_raw_transaction(&self, transaction_bytes: String) -> Result<TransactionInfo, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/sendtransaction.md")]
    async fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/validaterawtransaction.md")]
    async fn validate_raw_transaction(&self, transaction_bytes: String) -> Result<bool, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getconnectioncount.md")]
    async fn get_connection_count(&self) -> Result<usize, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getpeerinfo.md")]
    async fn get_peer_info(&self) -> Result<PeerInfo, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getnodeinfo.md")]
    async fn get_node_info(&self) -> Result<NodeInfo, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getnodestats.md")]
    async fn get_node_stats(&self) -> Result<NodeStats, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getblocktemplate.md")]
    async fn get_block_template(&self) -> Result<BlockTemplate, RpcError>;

    #[doc = include_str!("../documentation/public_endpoints/getnetworkgraph.md")]
    async fn get_network_graph(&self) -> Result<NetworkGraph, RpcError>;
}

/// Definition of private RPC endpoints that require authentication.
#[async_trait::async_trait]
pub trait ProtectedRpcFunctions {
    #[doc = include_str!("../documentation/private_endpoints/createaccount.md")]
    async fn create_account(&self) -> Result<RpcAccount, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/createrawtransaction.md")]
    async fn create_raw_transaction(
        &self,
        transaction_input: TransactionInputs,
    ) -> Result<CreateRawTransactionOuput, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/createtransactionkernel.md")]
    async fn create_transaction_kernel(&self, transaction_input: TransactionInputs) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/createtransaction.md")]
    async fn create_transaction(
        &self,
        private_keys: [String; 2], // TODO (howardwu): Genericize this.
        transaction_kernel: String,
    ) -> Result<CreateRawTransactionOuput, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/getrecordcommitments.md")]
    async fn get_record_commitments(&self) -> Result<Vec<String>, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/getrecordcommitmentcount.md")]
    async fn get_record_commitment_count(&self) -> Result<usize, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/getrawrecord.md")]
    async fn get_raw_record(&self, record_commitment: String) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/decoderecord.md")]
    async fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/decryptrecord.md")]
    async fn decrypt_record(&self, decryption_input: DecryptRecordInput) -> Result<String, RpcError>;

    #[doc = include_str!("../documentation/private_endpoints/disconnect.md")]
    async fn disconnect(&self, address: SocketAddr);

    #[doc = include_str!("../documentation/private_endpoints/connect.md")]
    async fn connect(&self, addresses: Vec<SocketAddr>);
}
