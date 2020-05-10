use crate::objects::TransactionError;

use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum BlockError {
    #[error("block already exists {}", _0)]
    BlockExists(String),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    TransactionError(TransactionError),

    #[error("block number {} has not been mined yet", _0)]
    InvalidBlockNumber(u32),

    #[error("expected block parent: {} got parent: {} ", _0, _1)]
    InvalidParent(String, String),

    #[error("the given block {} is not a canonical or sidechain block", _0)]
    IrrelevantBlock(String),
}

impl From<std::io::Error> for BlockError {
    fn from(error: std::io::Error) -> Self {
        BlockError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<TransactionError> for BlockError {
    fn from(error: TransactionError) -> Self {
        BlockError::TransactionError(error)
    }
}
