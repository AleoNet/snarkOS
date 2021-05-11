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

//! Structures for RPC endpoint requests and responses.

use chrono::{DateTime, Utc};
use jsonrpc_http_server::jsonrpc_core::Metadata;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Defines the authentication format for accessing private endpoints on the RPC server
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RpcCredentials {
    /// The username in the credential
    pub username: String,
    /// The password in the credential
    pub password: String,
}

/// RPC metadata for encoding authentication
#[derive(Default, Clone)]
pub struct Meta {
    /// An optional authentication string for protected RPC functions
    pub auth: Option<String>,
}

impl Metadata for Meta {}

/// Returned value for the `getblock` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block Hash
    pub hash: String,

    /// Block Height
    pub height: Option<u32>,

    /// Number of confirmations
    pub confirmations: u32,

    /// Block Size
    pub size: usize,

    /// Previous block hash
    pub previous_block_hash: String,

    /// Merkle root representing the transactions in the block
    pub merkle_root: String,

    /// Merkle root of the transactions in the block using a Pedersen hash
    pub pedersen_merkle_root_hash: String,

    /// Proof of Succinct Work
    pub proof: String,

    /// Block time
    pub time: i64,

    /// Block difficulty target
    pub difficulty_target: u64,

    /// Nonce
    pub nonce: u32,

    /// List of transaction ids
    pub transactions: Vec<String>,
}

/// Returned value for the `getblocktemplate` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockTemplate {
    /// Previous block hash
    pub previous_block_hash: String,

    /// Block height
    pub block_height: u32,

    /// Block timestamp
    pub time: i64,

    /// Proof of work difficulty target
    pub difficulty_target: u64,

    /// Transactions to include in the block (excluding the coinbase transaction)
    pub transactions: Vec<String>,

    /// Amount spendable by the coinbase transaction (block rewards + transaction fees)
    pub coinbase_value: u64,
}

/// Output for the `createrawtransaction` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateRawTransactionOuput {
    /// The newly created transaction from calling the `createrawtransaction` endpoint
    pub encoded_transaction: String,
    /// The newly created records from calling the `createrawtransaction` endpoint
    pub encoded_records: Vec<String>,
}

/// Input for the `decryptrecord` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DecryptRecordInput {
    /// The encrypted record
    pub encrypted_record: String,

    /// The account view key used to decrypt the record
    pub account_view_key: String,
}

/// Returned value for the `getnodeinfo` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Flag indicating if the node is operating as a miner
    pub is_miner: bool,

    /// Flag indicating if the node is currently syncing
    pub is_syncing: bool,

    /// The timestamp of when the node was launched.
    pub launched: DateTime<Utc>,

    /// The version of the client binary.
    pub version: String,
}

/// Returned value for the `getnodestats` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeStats {
    /// Stats related to messages received by the node.
    pub inbound: NodeInboundStats,
    /// Stats related to messages sent by the node.
    pub outbound: NodeOutboundStats,
    /// Stats related to the node's connections.
    pub connections: NodeConnectionStats,
    /// Stats related to the node's handshakes.
    pub handshakes: NodeHandshakeStats,
    /// Stats related to the node's queues.
    pub queues: NodeQueueStats,
    /// Miscellaneous stats related to the node.
    pub misc: NodeMiscStats,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeInboundStats {
    /// The number of successfully processed inbound messages.
    pub all_successes: u64,
    /// The number of inbound messages that couldn't be processed.
    pub all_failures: u64,

    /// The number of all received `Block` messages.
    pub blocks: u64,
    /// The number of all received `GetBlocks` messages.
    pub getblocks: u64,
    /// The number of all received `GetMemoryPool` messages.
    pub getmemorypool: u64,
    /// The number of all received `GetPeers` messages.
    pub getpeers: u64,
    /// The number of all received `GetSync` messages.
    pub getsync: u64,
    /// The number of all received `MemoryPool` messages.
    pub memorypool: u64,
    /// The number of all received `Peers` messages.
    pub peers: u64,
    /// The number of all received `Ping` messages.
    pub pings: u64,
    /// The number of all received `Pong` messages.
    pub pongs: u64,
    /// The number of all received `Sync` messages.
    pub syncs: u64,
    /// The number of all received `SyncBlock` messages.
    pub syncblocks: u64,
    /// The number of all received `Transaction` messages.
    pub transactions: u64,
    /// The number of all received `Unknown` messages.
    pub unknown: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeOutboundStats {
    /// The number of messages successfully sent by the node.
    pub all_successes: u64,
    /// The number of messages that failed to be sent to peers.
    pub all_failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeConnectionStats {
    /// The number of all connections the node has accepted.
    pub all_accepted: u64,
    /// The number of all connections the node has initiated.
    pub all_initiated: u64,
    /// The number of rejected inbound connection requests.
    pub all_rejected: u64,
    /// Number of currently connected peers.
    pub connected_peers: u16,
    /// Number of currently connecting peers.
    pub connecting_peers: u16,
    /// Number of known disconnected peers.
    pub disconnected_peers: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeHandshakeStats {
    /// The number of failed handshakes as the initiator.
    pub failures_init: u64,
    /// The number of failed handshakes as the responder.
    pub failures_resp: u64,
    /// The number of successful handshakes as the initiator.
    pub successes_init: u64,
    /// The number of successful handshakes as the responder.
    pub successes_resp: u64,
    /// The number of handshake timeouts as the initiator.
    pub timeouts_init: u64,
    /// The number of handshake timeouts as the responder.
    pub timeouts_resp: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeQueueStats {
    /// The number of messages queued in the common inbound channel.
    pub inbound: u32,
    /// The number of messages queued in the individual outbound channels.
    pub outbound: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NodeMiscStats {
    /// The current block height of the node.
    pub block_height: u32,
    /// The number of blocks the node has mined.
    pub blocks_mined: u32,
    /// The number of duplicate blocks received.
    pub duplicate_blocks: u64,
    /// The number of duplicate sync blocks received.
    pub duplicate_sync_blocks: u64,
}

/// Returned value for the `getpeerinfo` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerInfo {
    /// The peers connected to this node
    pub peers: Vec<SocketAddr>,
}

/// Record payload data
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCRecordPayload {
    /// Record payload
    pub payload: String,
}

/// Returned value for the `decoderawrecord` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordInfo {
    /// The record owner
    pub owner: String,

    /// Record is dummy flag
    pub is_dummy: bool,

    /// Record value
    pub value: u64,

    /// Record payload
    pub payload: RPCRecordPayload,

    /// Record birth program id
    pub birth_program_id: String,

    /// Record death program id
    pub death_program_id: String,

    /// Record serial number nonce
    pub serial_number_nonce: String,

    /// Record commitment
    pub commitment: String,

    /// Record commitment randomness
    pub commitment_randomness: String,
}

/// Output for the `createaccount` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RpcAccount {
    /// An account private key
    pub private_key: String,
    /// An account view key corresponding to the account private key
    pub view_key: String,
    /// An account address corresponding to the account private key
    pub address: String,
}

/// Returned value for the `gettransaction` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction id
    pub txid: String,

    /// Transaction size
    pub size: usize,

    /// Transaction inputs
    pub old_serial_numbers: Vec<String>,

    /// Transaction outputs
    pub new_commitments: Vec<String>,

    /// Transaction Memo
    pub memo: String,

    /// Network id of the transaction
    pub network_id: u8,

    /// Merkle tree digest
    pub digest: String,

    /// Transaction (outer snark) proof
    pub transaction_proof: String,

    /// Program verification key commitment
    pub program_commitment: String,

    /// Local data root
    pub local_data_root: String,

    /// Transaction value balance
    pub value_balance: i64,

    /// Transaction signatures (Delegated DPC)
    pub signatures: Vec<String>,

    /// Encrypted records
    pub encrypted_records: Vec<String>,

    /// Block the transaction lives in
    pub transaction_metadata: TransactionMetadata,
}

/// Input for the `createrawtransaction` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionInputs {
    /// Encoded records that are being spent
    pub old_records: Vec<String>,

    /// Account private keys owning the spent records
    pub old_account_private_keys: Vec<String>,

    /// Transaction recipent and amounts
    pub recipients: Vec<TransactionRecipient>,

    /// Transaction memo
    pub memo: Option<String>,

    /// Network id of the transaction
    pub network_id: u8,
    // Attributes that will be relevant for custom programs
    //    pub new_birth_programs: Vec<String>,
    //    pub new_death_programs: Vec<String>,
    //    pub new_payloads: Vec<String>,
}

/// Additional metadata included with a transaction response
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionMetadata {
    /// The block number associated with this transaction
    pub block_number: Option<u32>,
}

/// Recipient of a transaction
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionRecipient {
    /// The recipient's account address
    pub address: String,
    /// The amount being sent
    pub amount: u64,
}
