use bincode;
use rocksdb;
use std::fmt::Debug;

#[derive(Debug, Fail)]
pub enum StorageError {
    #[fail(display = "block already exists {:?}", _0)]
    BlockExists([u8; 32]),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "there is a double spend occuring with this transaction {}", _0)]
    DoubleSpend(String),

    #[fail(display = "invalid block number: {}", _0)]
    InvalidBlockNumber(u32),

    #[fail(
        display = "invalid number of blocks to remove {}. There are only {} existing blocks",
        _0, _1
    )]
    InvalidBlockRemovalNum(u32, u32),

    #[fail(display = "invalid block with hash {}", _0)]
    InvalidBlockHash(String),

    #[fail(display = "invalid column family {}", _0)]
    InvalidColumnFamily(u32),

    #[fail(display = "invalid next block: latest hash: {:?} parent: {:?} ", _0, _1)]
    InvalidNextBlock(String, String),

    #[fail(display = "invalid block with parent hash {}", _0)]
    InvalidParentHash(String),

    #[fail(display = "missing transaction with id {}", _0)]
    InvalidTransactionId(String),

    #[fail(display = "missing transaction meta with id {}", _0)]
    InvalidTransactionMeta(String),

    #[fail(display = "the given block is irrelevant")]
    IrrelevantBlock,

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "missing value given key {}", _0)]
    MissingValue(String),

    #[fail(display = "missing parent block for block hash {:?}", _0)]
    MissingParentBlock([u8; 32]),

    #[fail(display = "Null Error {:?}", _0)]
    NullError(()),
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
