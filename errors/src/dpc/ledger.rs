use crate::{algorithms::MerkleError, objects::TransactionError, storage::StorageError};

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("duplicate sn pushed to ledger")]
    DuplicateMemo,

    #[error("duplicate memo pushed to ledger")]
    DuplicateSn,

    #[error("database already exists")]
    ExistingDatabase,

    #[error("invalid cm pushed to ledger")]
    InvalidCm,

    #[error("invalid cm index during proving")]
    InvalidCmIndex,

    #[error("{}", _0)]
    MerkleError(MerkleError),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    StorageError(StorageError),

    #[error("{}", _0)]
    TransactionError(TransactionError),
}

impl From<std::io::Error> for LedgerError {
    fn from(error: std::io::Error) -> Self {
        LedgerError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<MerkleError> for LedgerError {
    fn from(error: MerkleError) -> Self {
        LedgerError::MerkleError(error)
    }
}

impl From<StorageError> for LedgerError {
    fn from(error: StorageError) -> Self {
        LedgerError::StorageError(error)
    }
}

impl From<TransactionError> for LedgerError {
    fn from(error: TransactionError) -> Self {
        LedgerError::TransactionError(error)
    }
}
