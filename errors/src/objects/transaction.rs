use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("UTXO has already been spent {:?} index: {:?}", _0, _1)]
    AlreadySpent(Vec<u8>, u32),

    #[error("there is a double spend occuring with this transaction {}", _0)]
    DoubleSpend(String),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("insufficient funds from input: {} to spend as output: {}", _0, _1)]
    InsufficientFunds(u64, u64),

    #[error("invalid coinbase transaction")]
    InvalidCoinbaseTransaction,

    #[error("invalid transaction id {:?}", _0)]
    InvalidTransactionId(usize),

    #[error("invalid variable size integer: {:?}", _0)]
    InvalidVariableSizeInteger(usize),

    #[error("{}", _0)]
    Message(String),

    #[error("missing outpoint script public key")]
    MissingOutpointScriptPublicKey,

    #[error("the block has multiple coinbase transactions: {:?}", _0)]
    MultipleCoinbaseTransactions(u32),

    #[error("Null Error {:?}", _0)]
    NullError(()),
}

impl From<hex::FromHexError> for TransactionError {
    fn from(error: hex::FromHexError) -> Self {
        TransactionError::Crate("hex", format!("{:?}", error))
    }
}

impl From<std::io::Error> for TransactionError {
    fn from(error: std::io::Error) -> Self {
        TransactionError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<std::num::ParseIntError> for TransactionError {
    fn from(error: std::num::ParseIntError) -> Self {
        TransactionError::Crate("std::num", format!("{:?}", error))
    }
}

impl From<std::str::ParseBoolError> for TransactionError {
    fn from(error: std::str::ParseBoolError) -> Self {
        TransactionError::Crate("std::str", format!("{:?}", error))
    }
}

impl From<()> for TransactionError {
    fn from(_error: ()) -> Self {
        TransactionError::NullError(())
    }
}

impl From<&'static str> for TransactionError {
    fn from(msg: &'static str) -> Self {
        TransactionError::Message(msg.into())
    }
}
