use crate::{
    algorithms::MerkleError,
    objects::{BlockError, TransactionError},
    parameters::ParametersError,
};

use bincode;
use rocksdb;
use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("duplicate commitment")]
    DuplicateCm,

    #[error("duplicate serial number")]
    DuplicateSn,

    #[error("duplicate transaction memo")]
    DuplicateMemo,

    #[error("existing record commitment {:?}", _0)]
    ExistingCm(Vec<u8>),

    #[error("existing transaction memo {:?}", _0)]
    ExistingMemo(Vec<u8>),

    #[error("existing serial number {:?}", _0)]
    ExistingSn(Vec<u8>),

    #[error("invalid number of blocks to remove {}. There are only {} existing blocks", _0, _1)]
    InvalidBlockRemovalNum(u32, u32),

    #[error("invalid column family {}", _0)]
    InvalidColumnFamily(u32),

    #[error("missing outpoint with transaction with id {} and index {}", _0, _1)]
    InvalidOutpoint(String, usize),

    #[error("missing transaction with id {}", _0)]
    InvalidTransactionId(String),

    #[error("{}", _0)]
    Message(String),

    #[error("missing block hash value given block number {}", _0)]
    MissingBlockHash(u32),

    #[error("missing block header value given block hash {}", _0)]
    MissingBlockHeader(String),

    #[error("missing block number value given block hash {}", _0)]
    MissingBlockNumber(String),

    #[error("missing block transactions value for block hash {}", _0)]
    MissingBlockTransactions(String),

    #[error("missing child block hashes value for block hash {}", _0)]
    MissingChildBlock(String),

    #[error("missing current commitment index")]
    MissingCurrentCmIndex,

    #[error("missing current merkle tree digest")]
    MissingCurrentDigest,

    #[error("missing current memo index")]
    MissingCurrentMemoIndex,

    #[error("missing current serial number index")]
    MissingCurrentSnIndex,

    #[error("missing genesis address")]
    MissingGenesisAccount,

    #[error("missing genesis commitment")]
    MissingGenesisCm,

    #[error("missing genesis memo")]
    MissingGenesisMemo,

    #[error("missing genesis predicate vk bytes")]
    MissingGenesisPredVkBytes,

    #[error("missing genesis serial number")]
    MissingGenesisSn,

    #[error("missing transaction meta value for transaction id {}", _0)]
    MissingTransactionMeta(String),

    #[error("missing value given key {}", _0)]
    MissingValue(String),

    #[error("Null Error {:?}", _0)]
    NullError(()),

    #[error("{}", _0)]
    BlockError(BlockError),

    #[error("{}", _0)]
    MerkleError(MerkleError),

    #[error("{}", _0)]
    ParametersError(ParametersError),

    #[error("{}", _0)]
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

impl From<ParametersError> for StorageError {
    fn from(error: ParametersError) -> Self {
        StorageError::ParametersError(error)
    }
}

impl From<TransactionError> for StorageError {
    fn from(error: TransactionError) -> Self {
        StorageError::TransactionError(error)
    }
}
