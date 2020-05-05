use crate::{
    algorithms::CRHError,
    dpc::DPCError,
    objects::{BlockError, TransactionError},
    storage::StorageError,
};

use std::fmt::Debug;

/// Possible block verification errors
#[derive(Debug, Fail)]
pub enum ConsensusError {
    #[fail(display = "UTXO has already been spent {:?} index: {:?}", _0, _1)]
    AlreadySpent(Vec<u8>, u32),

    #[fail(display = "{}", _0)]
    BlockError(BlockError),

    #[fail(display = "Block is too large: {}. Exceeds {} maximum", _0, _1)]
    BlockTooLarge(usize, usize),

    #[fail(display = "A coinbase transaction already exists in the block")]
    CoinbaseTransactionAlreadyExists(),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    CRHError(CRHError),

    #[fail(display = "{}", _0)]
    DPCError(DPCError),

    #[fail(display = "timestamp more than 2 hours into the future {:?} actual {:?}", _0, _1)]
    FuturisticTimestamp(i64, i64),

    #[fail(display = "invalid coinbase transaction")]
    InvalidCoinbaseTransaction,

    #[fail(display = "block transactions do not hash to merkle root {:?}", _0)]
    MerkleRoot(String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "the block has multiple coinbase transactions: {:?}", _0)]
    MultipleCoinbaseTransactions(u32),

    #[fail(display = "nonce {:?} is greater than nonce limit {:?}", _0, _1)]
    NonceInvalid(u32, u32),

    #[fail(display = "all nonces have been tried for the current block header")]
    NonceLimitError,

    #[fail(display = "Missing genesis block")]
    NoGenesisBlock,

    #[fail(display = "expected {:?} actual {:?}", _0, _1)]
    NoParent(String, String),

    #[fail(display = "header greater than difficulty target {:?} actual {:?}", _0, _1)]
    PowInvalid(u64, u64),

    #[fail(display = "{}", _0)]
    StorageError(StorageError),

    #[fail(display = "timestamp {:?} is less than parent timestamp {:?}", _0, _1)]
    TimestampInvalid(i64, i64),

    #[fail(display = "{}", _0)]
    TransactionError(TransactionError),

    #[fail(display = "Transactions are spending more funds than they have available")]
    TransactionOverspending,
}

impl From<BlockError> for ConsensusError {
    fn from(error: BlockError) -> Self {
        ConsensusError::BlockError(error)
    }
}

impl From<CRHError> for ConsensusError {
    fn from(error: CRHError) -> Self {
        ConsensusError::CRHError(error)
    }
}

impl From<DPCError> for ConsensusError {
    fn from(error: DPCError) -> Self {
        ConsensusError::DPCError(error)
    }
}

impl From<StorageError> for ConsensusError {
    fn from(error: StorageError) -> Self {
        ConsensusError::StorageError(error)
    }
}

impl From<TransactionError> for ConsensusError {
    fn from(error: TransactionError) -> Self {
        ConsensusError::TransactionError(error)
    }
}

impl From<bincode::Error> for ConsensusError {
    fn from(error: bincode::Error) -> Self {
        ConsensusError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<std::io::Error> for ConsensusError {
    fn from(error: std::io::Error) -> Self {
        ConsensusError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<ConsensusError> for Box<dyn std::error::Error> {
    fn from(error: ConsensusError) -> Self {
        error.into()
    }
}
