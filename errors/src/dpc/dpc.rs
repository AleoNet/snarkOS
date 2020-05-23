use crate::{
    dpc::{BindingSignatureError, LedgerError},
    objects::AccountError,
};
use snarkvm_errors::algorithms::{CRHError, CommitmentError, PRFError, SNARKError, SignatureError};

#[derive(Debug, Error)]
pub enum DPCError {
    #[error("{}", _0)]
    AccountError(AccountError),

    #[error("{}", _0)]
    BindingSignatureError(BindingSignatureError),

    #[error("{}", _0)]
    CommitmentError(CommitmentError),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    LedgerError(LedgerError),

    #[error("{}", _0)]
    Message(String),

    #[error("missing inner snark proving parameters")]
    MissingInnerSnarkProvingParameters,

    #[error("missing outer snark proving parameters")]
    MissingOuterSnarkProvingParameters,

    #[error("{}", _0)]
    PRFError(PRFError),

    #[error("{}", _0)]
    SignatureError(SignatureError),

    #[error("{}", _0)]
    SNARKError(SNARKError),
}

impl From<AccountError> for DPCError {
    fn from(error: AccountError) -> Self {
        DPCError::AccountError(error)
    }
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

impl From<SignatureError> for DPCError {
    fn from(error: SignatureError) -> Self {
        DPCError::SignatureError(error)
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
