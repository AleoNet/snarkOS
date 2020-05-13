use crate::{
    algorithms::CRHError,
    dpc::DPCError,
    objects::{BlockError, TransactionError},
    storage::StorageError,
};

use std::fmt::Debug;

/// Possible block verification errors
#[derive(Debug, Error)]
pub enum ConsensusError {
    #[error("UTXO has already been spent {:?} index: {:?}", _0, _1)]
    AlreadySpent(Vec<u8>, u32),

    #[error("{}", _0)]
    BlockError(BlockError),

    #[error("Block is too large: {}. Exceeds {} maximum", _0, _1)]
    BlockTooLarge(usize, usize),

    #[error("A coinbase transaction already exists in the block")]
    CoinbaseTransactionAlreadyExists(),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}", _0)]
    DPCError(DPCError),

    #[error("timestamp more than 2 hours into the future {:?} actual {:?}", _0, _1)]
    FuturisticTimestamp(i64, i64),

    #[error("invalid block {:?}", _0)]
    InvalidBlock(Vec<u8>),

    #[error("invalid coinbase transaction")]
    InvalidCoinbaseTransaction,

    #[error("block transactions do not hash to merkle root {:?}", _0)]
    MerkleRoot(String),

    #[error(
        "block transactions do not hash to the correct pedersen hash to merkle root {:?}",
        _0
    )]
    PedersenMerkleRoot(String),

    #[error("{}", _0)]
    Message(String),

    #[error("the block has multiple coinbase transactions: {:?}", _0)]
    MultipleCoinbaseTransactions(u32),

    #[error("nonce {:?} is greater than nonce limit {:?}", _0, _1)]
    NonceInvalid(u32, u32),

    #[error("all nonces have been tried for the current block header")]
    NonceLimitError,

    #[error("Missing genesis block")]
    NoGenesisBlock,

    #[error("expected {:?} actual {:?}", _0, _1)]
    NoParent(String, String),

    #[error("header greater than difficulty target {:?} actual {:?}", _0, _1)]
    PowInvalid(u64, u64),

    #[error("{}", _0)]
    StorageError(StorageError),

    #[error("timestamp {:?} is less than parent timestamp {:?}", _0, _1)]
    TimestampInvalid(i64, i64),

    #[error("{}", _0)]
    TransactionError(TransactionError),

    #[error("Transactions are spending more funds than they have available")]
    TransactionOverspending,

    #[error(transparent)]
    SynthesisError(#[from] crate::gadgets::SynthesisError),

    #[error("POSW Verification failed")]
    PoswVerificationFailed,

    #[error(transparent)]
    ConstraintFieldError(#[from] crate::curves::ConstraintFieldError),
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
