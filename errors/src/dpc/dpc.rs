use crate::{
    algorithms::{CRHError, CommitmentError, PRFError, SNARKError},
    dpc::{BindingSignatureError, LedgerError},
};

#[derive(Debug, Fail)]
pub enum DPCError {
    #[fail(display = "{}", _0)]
    BindingSignatureError(BindingSignatureError),

    #[fail(display = "{}", _0)]
    CommitmentError(CommitmentError),

    #[fail(display = "{}", _0)]
    CRHError(CRHError),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    LedgerError(LedgerError),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    PRFError(PRFError),

    #[fail(display = "{}", _0)]
    SNARKError(SNARKError),
}

impl From<BindingSignatureError> for DPCError {
    fn from(error: BindingSignatureError) -> Self {
        DPCError::BindingSignatureError(error)
    }
}

impl From<CommitmentError> for DPCError {
    fn from(error: CommitmentError) -> Self {
        DPCError::CommitmentError(error)
    }
}

impl From<CRHError> for DPCError {
    fn from(error: CRHError) -> Self {
        DPCError::CRHError(error)
    }
}

impl From<LedgerError> for DPCError {
    fn from(error: LedgerError) -> Self {
        DPCError::LedgerError(error)
    }
}

impl From<PRFError> for DPCError {
    fn from(error: PRFError) -> Self {
        DPCError::PRFError(error)
    }
}

impl From<SNARKError> for DPCError {
    fn from(error: SNARKError) -> Self {
        DPCError::SNARKError(error)
    }
}

impl From<std::io::Error> for DPCError {
    fn from(error: std::io::Error) -> Self {
        DPCError::Crate("std::io", format!("{:?}", error))
    }
}
