use snarkos_errors::storage::StorageError;
use snarkos_objects::{BlockHeader, BlockHeaderHash};

use bincode;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

pub const COL_META: u32 = 0;
pub const COL_BLOCK: u32 = 1;
pub const COL_BLOCK_HASHES: u32 = 2;
pub const COL_BLOCK_NUMBERS: u32 = 3;
pub const COL_BLOCK_TRANSACTIONS: u32 = 4;
pub const COL_TRANSACTIONS: u32 = 5;
pub const COL_TRANSACTION_META: u32 = 6;
pub const COL_CHILD_HASHES: u32 = 7;

pub const NUM_COLS: u32 = 8;

pub const KEY_BEST_BLOCK_NUMBER: &str = "BEST_BLOCK_NUMBER";
pub const KEY_MEMORY_POOL: &str = "MEMORY_POOL";
//pub const KEY_BEST_BLOCK_HASH: &'static str = "BEST_BLOCK_HASH";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub spent: Vec<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TransactionValue {
    pub count: u8,
    pub transaction_bytes: Vec<u8>,
}

impl TransactionValue {
    pub fn new(transaction_bytes: Vec<u8>) -> Self {
        Self {
            count: 1,
            transaction_bytes,
        }
    }

    pub fn increment(self) -> Self {
        Self {
            count: self.count + 1,
            transaction_bytes: self.transaction_bytes,
        }
    }

    pub fn decrement(self) -> Self {
        Self {
            count: self.count - 1,
            transaction_bytes: self.transaction_bytes,
        }
    }
}

// Key, value pair
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum KeyValue {
    /// Meta data
    Meta(&'static str, Vec<u8>),
    /// block_header_hash to block
    BlockHeaders(BlockHeaderHash, BlockHeader),
    /// block number -> block hash
    BlockHashes(u32, BlockHeaderHash),
    /// block_hash -> block number
    BlockNumbers(BlockHeaderHash, u32),
    /// block_hash -> list of transaction ids
    BlockTransactions(BlockHeaderHash, Vec<Vec<u8>>),
    /// transaction id -> Transaction hex
    Transactions(Vec<u8>, TransactionValue),
    /// transaction id -> TransactionMeta
    TransactionMeta(Vec<u8>, TransactionMeta),
    /// parent_block_hash -> child_block_hash
    ChildHashes(BlockHeaderHash, BlockHeaderHash),
}

// Key
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Key {
    /// Meta data
    Meta(&'static str),
    /// block header hash
    BlockHeaders(BlockHeaderHash),
    /// block nu ber -> block hash
    BlockHashes(u32),
    /// block_hash -> block number
    BlockNumbers(BlockHeaderHash),
    /// block_hash
    BlockTransactions(BlockHeaderHash),
    /// transaction id
    Transactions(Vec<u8>),
    /// transaction id
    TransactionMeta(Vec<u8>),
    /// parent_block_hash
    ChildHashes(BlockHeaderHash),
}

impl Display for Key {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Value
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Value {
    /// Meta data
    Meta(Vec<u8>),
    /// block number to block
    BlockHeaders(BlockHeader),
    /// block number -> block hash
    BlockHashes(BlockHeaderHash),
    /// block_hash -> block number
    BlockNumbers(u32),
    /// list of transactions ids
    BlockTransactions(Vec<Vec<u8>>),
    /// Transaction hex
    Transactions(TransactionValue),
    /// Transaction Meta
    TransactionMeta(TransactionMeta),
    /// child_block_hash
    ChildHashes(BlockHeaderHash),
}

impl Value {
    pub fn from_bytes(key: &Key, bytes: &Vec<u8>) -> Result<Self, StorageError> {
        Ok(match *key {
            Key::Meta(_) => Value::Meta(bytes.clone()),
            Key::BlockHeaders(_) => Value::BlockHeaders(bincode::deserialize(&bytes)?),
            Key::BlockHashes(_) => Value::BlockHashes(bincode::deserialize(&bytes)?),
            Key::BlockNumbers(_) => Value::BlockNumbers(bytes_to_u32(bytes.clone())),
            Key::BlockTransactions(_) => Value::BlockTransactions(bincode::deserialize(bytes)?),
            Key::Transactions(_) => Value::Transactions(bincode::deserialize(bytes)?),
            Key::TransactionMeta(_) => Value::TransactionMeta(bincode::deserialize(bytes)?),
            Key::ChildHashes(_) => Value::ChildHashes(bincode::deserialize(&bytes)?),
        })
    }

    pub fn meta(self) -> Option<Vec<u8>> {
        match self {
            Value::Meta(bytes) => Some(bytes),
            _ => None,
        }
    }

    pub fn block_header(self) -> Option<BlockHeader> {
        match self {
            Value::BlockHeaders(block_hash) => Some(block_hash),
            _ => None,
        }
    }

    pub fn block_hash(self) -> Option<BlockHeaderHash> {
        match self {
            Value::BlockHashes(block_header) => Some(block_header),
            _ => None,
        }
    }

    pub fn block_number(self) -> Option<u32> {
        match self {
            Value::BlockNumbers(block_number) => Some(block_number),
            _ => None,
        }
    }

    pub fn block_transaction(self) -> Option<Vec<Vec<u8>>> {
        match self {
            Value::BlockTransactions(transactions) => Some(transactions),
            _ => None,
        }
    }

    pub fn transactions(self) -> Option<TransactionValue> {
        match self {
            Value::Transactions(transaction_value) => Some(transaction_value),
            _ => None,
        }
    }

    pub fn transaction_meta(self) -> Option<TransactionMeta> {
        match self {
            Value::TransactionMeta(transaction_meta) => Some(transaction_meta),
            _ => None,
        }
    }

    pub fn child_hashes(self) -> Option<BlockHeaderHash> {
        match self {
            Value::ChildHashes(child_hash) => Some(child_hash),
            _ => None,
        }
    }
}

pub struct ColKey {
    pub column: u32,
    pub key: Vec<u8>,
}

impl<'a> From<&'a Key> for ColKey {
    fn from(i: &'a Key) -> Self {
        let (column, key) = match *i {
            Key::Meta(ref key) => (COL_META, bincode::serialize(key).unwrap()),
            Key::BlockHeaders(ref key) => (COL_BLOCK, bincode::serialize(key).unwrap()),
            Key::BlockHashes(ref key) => (COL_BLOCK_HASHES, key.to_le_bytes().to_vec()),
            Key::BlockNumbers(ref key) => (COL_BLOCK_NUMBERS, bincode::serialize(key).unwrap()),
            Key::BlockTransactions(ref key) => (COL_BLOCK_TRANSACTIONS, bincode::serialize(key).unwrap()),
            Key::Transactions(ref key) => (COL_TRANSACTIONS, key.clone()),
            Key::TransactionMeta(ref key) => (COL_TRANSACTION_META, key.clone()),
            Key::ChildHashes(ref key) => (COL_CHILD_HASHES, bincode::serialize(key).unwrap()),
        };

        Self { column, key }
    }
}

pub struct ColKeyValue {
    pub column: u32,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

impl<'a> From<&'a KeyValue> for ColKeyValue {
    fn from(i: &'a KeyValue) -> Self {
        let (column, key, value) = match *i {
            KeyValue::Meta(ref key, ref value) => (COL_META, bincode::serialize(key).unwrap(), value.clone()),
            KeyValue::BlockHeaders(ref key, ref value) => (
                COL_BLOCK,
                bincode::serialize(key).unwrap(),
                bincode::serialize(value).unwrap(),
            ),
            KeyValue::BlockHashes(ref key, ref value) => (
                COL_BLOCK_HASHES,
                key.to_le_bytes().to_vec(),
                bincode::serialize(value).unwrap(),
            ),
            KeyValue::BlockNumbers(ref key, ref value) => (
                COL_BLOCK_NUMBERS,
                bincode::serialize(key).unwrap(),
                value.to_le_bytes().to_vec(),
            ),
            KeyValue::BlockTransactions(ref key, ref value) => (
                COL_BLOCK_TRANSACTIONS,
                bincode::serialize(key).unwrap(),
                bincode::serialize(value).unwrap(),
            ),
            KeyValue::Transactions(ref key, ref value) => {
                (COL_TRANSACTIONS, key.clone(), bincode::serialize(value).unwrap())
            }
            KeyValue::TransactionMeta(ref key, ref value) => {
                (COL_TRANSACTION_META, key.clone(), bincode::serialize(value).unwrap())
            }
            KeyValue::ChildHashes(ref key, ref value) => (
                COL_CHILD_HASHES,
                bincode::serialize(key).unwrap(),
                bincode::serialize(value).unwrap(),
            ),
        };

        Self { column, key, value }
    }
}

pub fn bytes_to_u32(bytes: Vec<u8>) -> u32 {
    let mut num_bytes = [0u8; 4];
    num_bytes.copy_from_slice(&bytes);

    u32::from_le_bytes(num_bytes)
}
