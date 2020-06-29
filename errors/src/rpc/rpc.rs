use crate::{
    algorithms::CRHError,
    consensus::ConsensusError,
    dpc::DPCError,
    network::SendError,
    objects::{AccountError, BlockError, TransactionError},
    storage::StorageError,
};

use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum RpcError {
    #[error("{}", _0)]
    AccountError(AccountError),

    #[error("{}", _0)]
    BlockError(BlockError),

    #[error("{}", _0)]
    ConsensusError(ConsensusError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}", _0)]
    DPCError(DPCError),

    #[error("invalid block hash: {}", _0)]
    InvalidBlockHash(String),

    #[error("invalid metadata: {}", _0)]
    InvalidMetadata(String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    SendError(SendError),

    #[error("{}", _0)]
    StorageError(StorageError),

    #[error("{}", _0)]
    TransactionError(TransactionError),
}

impl From<AccountError> for RpcError {
    fn from(error: AccountError) -> Self {
        RpcError::AccountError(error)
    }
}

impl From<BlockError> for RpcError {
    fn from(error: BlockError) -> Self {
        RpcError::BlockError(error)
    }
}

impl From<ConsensusError> for RpcError {
    fn from(error: ConsensusError) -> Self {
        RpcError::ConsensusError(error)
    }
}

impl From<CRHError> for RpcError {
    fn from(error: CRHError) -> Self {
        RpcError::CRHError(error)
    }
}

impl From<DPCError> for RpcError {
    fn from(error: DPCError) -> Self {
        RpcError::DPCError(error)
    }
}

impl From<SendError> for RpcError {
    fn from(error: SendError) -> Self {
        RpcError::SendError(error)
    }
}

impl From<StorageError> for RpcError {
    fn from(error: StorageError) -> Self {
        RpcError::StorageError(error)
    }
}

impl From<TransactionError> for RpcError {
    fn from(error: TransactionError) -> Self {
        RpcError::TransactionError(error)
    }
}

impl From<hex::FromHexError> for RpcError {
    fn from(error: hex::FromHexError) -> Self {
        RpcError::Crate("hex", format!("{:?}", error))
    }
}

impl From<jsonrpc_core::Error> for RpcError {
    fn from(error: jsonrpc_core::Error) -> Self {
        RpcError::Crate("jsonrpc_core", format!("{:?}", error))
    }
}

impl From<std::io::Error> for RpcError {
    fn from(error: std::io::Error) -> Self {
        RpcError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<&'static str> for RpcError {
    fn from(msg: &'static str) -> Self {
        RpcError::Message(msg.into())
    }
}

impl From<std::boxed::Box<dyn std::any::Any + std::marker::Send>> for RpcError {
    fn from(error: std::boxed::Box<dyn std::any::Any + std::marker::Send>) -> Self {
        RpcError::Crate("std::boxed::Box", format!("{:?}", error))
    }
}

impl From<RpcError> for jsonrpc_core::Error {
    fn from(_error: RpcError) -> Self {
        jsonrpc_core::Error::invalid_request()
    }
}
