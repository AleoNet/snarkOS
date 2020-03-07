use crate::storage::StorageError;

use std::fmt::Debug;

#[derive(Debug, Fail)]
pub enum TransactionError {
    #[fail(display = "UTXO has already been spent {:?} index: {:?}", _0, _1)]
    AlreadySpent(Vec<u8>, u32),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "invalid coinbase transaction")]
    InvalidCoinbaseTransaction,

    #[fail(display = "insufficient funds from input: {} to spend as output: {}", _0, _1)]
    InsufficientFunds(u64, u64),

    #[fail(display = "missing outpoint with transaction with id {} and index {}", _0, _1)]
    InvalidOutpoint(String, usize),

    #[fail(display = "invalid pub key hash script_pub_key: {} script_sig: {}", _0, _1)]
    InvalidPubKeyHash(String, String),

    #[fail(display = "invalid script pub key for format: {}", _0)]
    InvalidScriptPubKey(String),

    #[fail(display = "invalid transaction id {:?}", _0)]
    InvalidTransactionId(usize),

    #[fail(display = "invalid transaction id {:?}", _0)]
    InvalidTransactionIdString(String),

    #[fail(display = "invalid variable size integer: {:?}", _0)]
    InvalidVariableSizeInteger(usize),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "missing outpoint script public key")]
    MissingOutpointScriptPublicKey,

    #[fail(display = "the block has multiple coinbase transactions: {:?}", _0)]
    MultipleCoinbaseTransactions(u32),

    #[fail(display = "Null Error {:?}", _0)]
    NullError(()),

    #[fail(display = "{}", _0)]
    StorageError(StorageError),
}

impl From<base58::FromBase58Error> for TransactionError {
    fn from(error: base58::FromBase58Error) -> Self {
        TransactionError::Crate("base58", format!("{:?}", error))
    }
}

impl From<hex::FromHexError> for TransactionError {
    fn from(error: hex::FromHexError) -> Self {
        TransactionError::Crate("hex", format!("{:?}", error))
    }
}

impl From<secp256k1::Error> for TransactionError {
    fn from(error: secp256k1::Error) -> Self {
        TransactionError::Crate("secp256k1", format!("{:?}", error))
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

impl From<wagyu_model::AddressError> for TransactionError {
    fn from(error: wagyu_model::AddressError) -> Self {
        TransactionError::Crate("wagyu_model", format!("{:?}", error))
    }
}

impl From<wagyu_model::PublicKeyError> for TransactionError {
    fn from(error: wagyu_model::PublicKeyError) -> Self {
        TransactionError::Crate("wagyu_model", format!("{:?}", error))
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

impl From<TransactionError> for Box<dyn std::error::Error> {
    fn from(error: TransactionError) -> Self {
        error.into()
    }
}

impl From<StorageError> for TransactionError {
    fn from(error: StorageError) -> Self {
        TransactionError::StorageError(error)
    }
}
