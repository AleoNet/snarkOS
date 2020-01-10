use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};

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

/// Transaction input
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionOutpoint {
    /// Previous transaction id
    pub txid: String,
    /// Previous transaction output index
    pub vout: u32,
}

/// Transaction input
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionInput {
    /// Previous transaction id
    pub txid: String,

    /// Previous transaction output index
    pub vout: u32,

    /// Script signature
    pub script_sig: String,
}

/// Transaction output
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionOutput {
    /// Transaction output amount
    pub amount: u64,

    /// Transaction output public key script
    pub script_pub_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RPCTransactionOutputs(pub HashMap<String, u64>);

/// Returned value for the `gettransaction` rpc call
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionInfo {
    /// Transaction id
    pub txid: String,

    /// Transaction size
    pub size: usize,

    /// Transaction version
    pub version: u32,

    /// Transaction inputs
    pub inputs: Vec<RPCTransactionInput>,

    /// Transaction outputs
    pub outputs: Vec<RPCTransactionOutput>,
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
