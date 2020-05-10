use crate::algorithms::{CRHError, CommitmentError, PRFError, SignatureError};

#[derive(Debug, Fail)]
pub enum AccountError {
    #[fail(display = "{}", _0)]
    CommitmentError(CommitmentError),

    #[fail(display = "{}", _0)]
    CRHError(CRHError),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    PRFError(PRFError),

    #[fail(display = "{}", _0)]
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
