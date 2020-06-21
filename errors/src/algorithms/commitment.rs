use std::io::{Error, ErrorKind};

use crate::algorithms::CRHError;

#[derive(Debug, Error)]
pub enum CommitmentError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("incorrect input length {} for window params {}x{}", _0, _1, _2)]
    IncorrectInputLength(usize, usize, usize),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}", _0)]
    Message(String),
}

impl From<CRHError> for CommitmentError {
    fn from(error: CRHError) -> Self {
        CommitmentError::CRHError(error)
    }
}

impl From<Error> for CommitmentError {
    fn from(error: Error) -> Self {
        CommitmentError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<CommitmentError> for Error {
    fn from(error: CommitmentError) -> Error {
        Error::new(ErrorKind::Other, error.to_string())
    }
}
