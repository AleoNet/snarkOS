use crate::algorithms::{CRHError, CommitmentError, PRFError, SignatureError};

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("{}", _0)]
    CommitmentError(CommitmentError),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    PRFError(PRFError),

    #[error("{}", _0)]
    SignatureError(SignatureError),
}

impl From<CommitmentError> for AccountError {
    fn from(error: CommitmentError) -> Self {
        AccountError::CommitmentError(error)
    }
}

impl From<CRHError> for AccountError {
    fn from(error: CRHError) -> Self {
        AccountError::CRHError(error)
    }
}

impl From<PRFError> for AccountError {
    fn from(error: PRFError) -> Self {
        AccountError::PRFError(error)
    }
}

impl From<SignatureError> for AccountError {
    fn from(error: SignatureError) -> Self {
        AccountError::SignatureError(error)
    }
}

impl From<std::io::Error> for AccountError {
    fn from(error: std::io::Error) -> Self {
        AccountError::Crate("std::io", format!("{:?}", error))
    }
}
