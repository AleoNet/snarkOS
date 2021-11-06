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

use crate::network::rpc::rpc_impl::RpcError;
use snarkvm::dpc::{Block, Network, RecordCiphertext, Transaction, Transition};

use std::net::SocketAddr;

/// Definition of public RPC endpoints.
#[async_trait::async_trait]
pub trait RpcFunctions<N: Network> {
    // #[doc = include_str!("../documentation/public_endpoints/latestblock.md")]
    async fn latest_block(&self) -> Result<Block<N>, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/latestblockheight.md")]
    async fn latest_block_height(&self) -> Result<u32, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/latestblockhash.md")]
    async fn latest_block_hash(&self) -> Result<N::BlockHash, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/getblock.md")]
    async fn get_block(&self, block_height: u32) -> Result<Block<N>, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/getblocks.md")]
    async fn get_blocks(&self, start_block_height: u32, end_block_height: u32) -> Result<Vec<Block<N>>, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/getblockheight.md")]
    async fn get_block_height(&self, block_hash: serde_json::Value) -> Result<u32, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/getblockhash.md")]
    async fn get_block_hash(&self, block_height: u32) -> Result<N::BlockHash, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/gettransaction.md")]
    async fn get_transaction(&self, transaction_id: serde_json::Value) -> Result<serde_json::Value, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/gettransition.md")]
    async fn get_transition(&self, transition_id: serde_json::Value) -> Result<Transition<N>, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/getciphertext.md")]
    async fn get_ciphertext(&self, ciphertext_id: serde_json::Value) -> Result<RecordCiphertext<N>, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/sendtransaction.md")]
    async fn send_transaction(&self, transaction_bytes: String) -> Result<N::TransactionID, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/ledgerproof.md")]
    async fn ledger_proof(&self, record_commitment: serde_json::Value) -> Result<String, RpcError>;

    // #[doc = include_str!("../documentation/public_endpoints/validaterawtransaction.md")]
    // async fn validate_raw_transaction(&self, transaction_bytes: String) -> Result<bool, RpcError>;
    //
    // #[doc = include_str!("../documentation/public_endpoints/getconnectioncount.md")]
    // async fn get_connection_count(&self) -> Result<usize, RpcError>;
}

// /// Definition of private RPC endpoints that require authentication.
// #[async_trait::async_trait]
// pub trait ProtectedRpcFunctions {
//     #[doc = include_str!("../documentation/private_endpoints/createaccount.md")]
//     async fn create_account(&self) -> Result<RpcAccount, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/createrawtransaction.md")]
//     async fn create_raw_transaction(
//         &self,
//         transaction_input: TransactionInputs,
//     ) -> Result<CreateRawTransactionOuput, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/createtransaction.md")]
//     async fn create_transaction(
//         &self,
//         private_keys: [String; 2], // TODO (howardwu): Genericize this.
//         transaction_kernel: String,
//     ) -> Result<CreateRawTransactionOuput, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/getrecordcommitments.md")]
//     async fn get_record_commitments(&self) -> Result<Vec<String>, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/getrecordcommitmentcount.md")]
//     async fn get_record_commitment_count(&self) -> Result<usize, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/getrawrecord.md")]
//     async fn get_raw_record(&self, record_commitment: String) -> Result<String, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/decoderecord.md")]
//     async fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/decryptrecord.md")]
//     async fn decrypt_record(&self, decryption_input: DecryptRecordInput) -> Result<String, RpcError>;
//
//     #[doc = include_str!("../documentation/private_endpoints/disconnect.md")]
//     async fn disconnect(&self, address: SocketAddr);
//
//     #[doc = include_str!("../documentation/private_endpoints/connect.md")]
//     async fn connect(&self, addresses: Vec<SocketAddr>);
// }
