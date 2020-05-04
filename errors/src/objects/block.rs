use crate::objects::TransactionError;

use std::fmt::Debug;

#[derive(Debug, Fail)]
pub enum BlockError {
    #[fail(display = "block already exists {}", _0)]
    BlockExists(String),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    TransactionError(TransactionError),

    #[fail(display = "block number {} has not been mined yet", _0)]
    InvalidBlockNumber(u32),

    #[fail(display = "expected block parent: {} got parent: {} ", _0, _1)]
    InvalidParent(String, String),

    #[fail(display = "the given block {} is not a canonical or sidechain block", _0)]
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

impl From<BlockError> for Box<dyn std::error::Error> {
    fn from(error: BlockError) -> Self {
        error.into()
    }
}
