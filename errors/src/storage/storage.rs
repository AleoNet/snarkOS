use crate::{
    algorithms::MerkleError,
    objects::{BlockError, TransactionError},
};

use bincode;
use rocksdb;
use std::fmt::Debug;

#[derive(Debug, Fail)]
pub enum StorageError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "duplicate commitment")]
    DuplicateCm,

    #[fail(display = "duplicate serial number")]
    DuplicateSn,

    #[fail(display = "duplicate transaction memo")]
    DuplicateMemo,

    #[fail(display = "existing record commitment {:?}", _0)]
    ExistingCm(Vec<u8>),

    #[fail(display = "existing transaction memo {:?}", _0)]
    ExistingMemo(Vec<u8>),

    #[fail(display = "existing serial number {:?}", _0)]
    ExistingSn(Vec<u8>),

    #[fail(
        display = "invalid number of blocks to remove {}. There are only {} existing blocks",
        _0, _1
    )]
    InvalidBlockRemovalNum(u32, u32),

    #[fail(display = "invalid column family {}", _0)]
    InvalidColumnFamily(u32),

    #[fail(display = "missing outpoint with transaction with id {} and index {}", _0, _1)]
    InvalidOutpoint(String, usize),

    #[fail(display = "missing transaction with id {}", _0)]
    InvalidTransactionId(String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "missing block hash value given block number {}", _0)]
    MissingBlockHash(u32),

    #[fail(display = "missing block header value given block hash {}", _0)]
    MissingBlockHeader(String),

    #[fail(display = "missing block number value given block hash {}", _0)]
    MissingBlockNumber(String),

    #[fail(display = "missing block transactions value for block hash {}", _0)]
    MissingBlockTransactions(String),

    #[fail(display = "missing child block hashes value for block hash {}", _0)]
    MissingChildBlock(String),

    #[fail(display = "missing current commitment index")]
    MissingCurrentCmIndex,

    #[fail(display = "missing current merkle tree digest")]
    MissingCurrentDigest,

    #[fail(display = "missing current memo index")]
    MissingCurrentMemoIndex,

    #[fail(display = "missing current serial number index")]
    MissingCurrentSnIndex,

    #[fail(display = "missing genesis address")]
    MissingGenesisAddress,

    #[fail(display = "missing genesis commitment")]
    MissingGenesisCm,

    #[fail(display = "missing genesis memo")]
    MissingGenesisMemo,

    #[fail(display = "missing genesis predicate vk bytes")]
    MissingGenesisPredVkBytes,

    #[fail(display = "missing genesis serial number")]
    MissingGenesisSn,

    #[fail(display = "missing transaction meta value for transaction id {}", _0)]
    MissingTransactionMeta(String),

    #[fail(display = "missing value given key {}", _0)]
    MissingValue(String),

    #[fail(display = "Null Error {:?}", _0)]
    NullError(()),

    #[fail(display = "{}", _0)]
    BlockError(BlockError),

    #[fail(display = "{}", _0)]
    MerkleError(MerkleError),

    #[fail(display = "{}", _0)]
    TransactionError(TransactionError),
}

impl From<bincode::Error> for StorageError {
    fn from(error: bincode::Error) -> Self {
        StorageError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<hex::FromHexError> for StorageError {
    fn from(error: hex::FromHexError) -> Self {
        StorageError::Crate("hex", format!("{:?}", error))
    }
}

impl From<rocksdb::Error> for StorageError {
    fn from(error: rocksdb::Error) -> Self {
        StorageError::Crate("rocksdb", format!("{:?}", error))
    }
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<()> for StorageError {
    fn from(_error: ()) -> Self {
        StorageError::NullError(())
    }
}

impl From<&'static str> for StorageError {
    fn from(msg: &'static str) -> Self {
        StorageError::Message(msg.into())
    }
}

impl From<StorageError> for Box<dyn std::error::Error> {
    fn from(error: StorageError) -> Self {
        error.into()
    }
}

impl From<BlockError> for StorageError {
    fn from(error: BlockError) -> Self {
        StorageError::BlockError(error)
    }
}

impl From<MerkleError> for StorageError {
    fn from(error: MerkleError) -> Self {
        StorageError::MerkleError(error)
    }
}

impl From<TransactionError> for StorageError {
    fn from(error: TransactionError) -> Self {
        StorageError::TransactionError(error)
    }
}
