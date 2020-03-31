use crate::{algorithms::MerkleError, objects::TransactionError};

#[derive(Debug, Fail)]
pub enum LedgerError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "Duplicate sn pushed to ledger")]
    DuplicateMemo,

    #[fail(display = "Duplicate memo pushed to ledger")]
    DuplicateSn,

    #[fail(display = "Invalid cm pushed to ledger")]
    InvalidCm,

    #[fail(display = "Invalid cm index during proving")]
    InvalidCmIndex,

    #[fail(display = "{}", _0)]
    MerkleError(MerkleError),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    TransactionError(TransactionError),
}

impl From<MerkleError> for LedgerError {
    fn from(error: MerkleError) -> Self {
        LedgerError::MerkleError(error)
    }
}

impl From<TransactionError> for LedgerError {
    fn from(error: TransactionError) -> Self {
        LedgerError::TransactionError(error)
    }
}
