use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

/// Returned value for the `getblock` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockInfo {
    /// Block Hash
    pub hash: String,

    /// Block Height
    pub height: u32,

    /// Number of confirmations
    pub confirmations: u32,

    /// Block Size
    pub size: usize,

    /// Nonce
    pub nonce: u32,

    /// Merkle Root
    pub merkle_root: String,

    /// List of transaction ids
    pub transactions: Vec<String>,

    /// Previous block hash
    pub previous_block_hash: String,

    /// Next block hash
    pub next_block_hash: String,
}

/// Returned value for the `getpeerinfo` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peers: Vec<SocketAddr>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionStuff {
    /// Merkle tree digest
    pub digest: String,

    /// Inner snark proof
    pub inner_proof: String,

    /// Predicate proof
    pub predicate_proof: String,

    /// Predicate verification key commitment
    pub predicate_commitment: String,

    /// Local data commitment
    pub local_data_commitment: String,

    /// Transaction value balance
    pub value_balance: i64,

    /// Transaction signatures (Delegated DPC)
    pub signatures: Vec<String>,
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

    /// DPC Stuff
    pub stuff: RPCTransactionStuff,
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
