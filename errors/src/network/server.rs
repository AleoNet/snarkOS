use crate::{
    network::{message::MessageError, ConnectError, SendError},
    objects::{BlockError, TransactionError},
    storage::StorageError,
};

#[derive(Debug, Fail)]
pub enum ServerError {
    #[fail(display = "{}", _0)]
    BlockError(BlockError),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    ConnectError(ConnectError),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    MessageError(MessageError),

    #[fail(display = "{}", _0)]
    SendError(SendError),

    #[fail(display = "{}", _0)]
    StorageError(StorageError),

    #[fail(display = "{}", _0)]
    TransactionError(TransactionError),
}

impl From<BlockError> for ServerError {
    fn from(error: BlockError) -> Self {
        ServerError::BlockError(error)
    }
}

impl From<ConnectError> for ServerError {
    fn from(error: ConnectError) -> Self {
        ServerError::ConnectError(error)
    }
}

impl From<MessageError> for ServerError {
    fn from(error: MessageError) -> Self {
        ServerError::MessageError(error)
    }
}

impl From<SendError> for ServerError {
    fn from(error: SendError) -> Self {
        ServerError::SendError(error)
    }
}

impl From<StorageError> for ServerError {
    fn from(error: StorageError) -> Self {
        ServerError::StorageError(error)
    }
}

impl From<TransactionError> for ServerError {
    fn from(error: TransactionError) -> Self {
        ServerError::TransactionError(error)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(error: std::io::Error) -> Self {
        ServerError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<std::net::AddrParseError> for ServerError {
    fn from(error: std::net::AddrParseError) -> Self {
        ServerError::Crate("std::net::AddrParseError", format!("{:?}", error))
    }
}
