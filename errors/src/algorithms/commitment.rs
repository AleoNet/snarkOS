use std::io::{Error, ErrorKind};

use crate::algorithms::CRHError;

#[derive(Debug, Fail)]
pub enum CommitmentError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    CRHError(CRHError),

    #[fail(display = "{}", _0)]
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
