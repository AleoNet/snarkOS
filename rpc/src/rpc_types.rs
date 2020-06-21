use jsonrpc_http_server::jsonrpc_core::Metadata;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Returned value for the `getblock` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RpcCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Default, Clone)]
pub struct Meta {
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

/// Returned value for the `getpeerinfo` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peers: Vec<SocketAddr>,
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

    /// Merkle tree digest
    pub digest: String,

    /// Transaction (outer snark) proof
    pub transaction_proof: String,

    /// Predicate verification key commitment
    pub predicate_commitment: String,

    /// Local data commitment
    pub local_data_commitment: String,

    /// Transaction value balance
    pub value_balance: i64,

    /// Transaction signatures (Delegated DPC)
    pub signatures: Vec<String>,

    /// Block the transaction lives in
    pub block_number: Option<u32>,
}

/// Record payload
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCRecordPayload {
    /// Record payload
    pub payload: String,
}

/// Returned value for the `decoderawrecord` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RecordInfo {
    /// Account public key of the record owner
    pub account_public_key: String,

    /// Record is dummy flag
    pub is_dummy: bool,

    /// Record value
    pub value: u64,

    /// Record payload
    pub payload: RPCRecordPayload,

    /// Record birth predicate bytes
    pub birth_predicate_repr: String,

    /// Record death predicate bytes
    pub death_predicate_repr: String,

    /// Record serial number nonce
    pub serial_number_nonce: String,

    /// Record commitment
    pub commitment: String,

    /// Record commitment randomness
    pub commitment_randomness: String,
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
    // Attributes that will be relevant for custom predicates
    //    pub new_birth_predicates: Vec<String>,
    //    pub new_death_predicates: Vec<String>,
    //    pub new_payloads: Vec<String>,
}

/// Recipient of a transaction
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionRecipient {
    /// Recipient account public key
    pub address: String,

    /// Amount being sent
    pub amount: u64,
}

/// Output for the `createrawtransaction` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateRawTransactionOuput {
    pub encoded_transaction: String,
    pub encoded_records: Vec<String>,
}
