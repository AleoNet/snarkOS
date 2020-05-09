use crate::network::message::{MessageError, MessageHeaderError};

use std::net::SocketAddr;

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    MessageHeaderError(MessageHeaderError),

    #[error("{}", _0)]
    MessageError(MessageError),

    #[error("Address {:?} not found", _0)]
    AddressNotFound(SocketAddr),
}

impl From<MessageError> for ConnectError {
    fn from(error: MessageError) -> Self {
        ConnectError::MessageError(error)
    }
}

impl From<MessageHeaderError> for ConnectError {
    fn from(error: MessageHeaderError) -> Self {
        ConnectError::MessageHeaderError(error)
    }
}

impl From<std::io::Error> for ConnectError {
    fn from(error: std::io::Error) -> Self {
        ConnectError::Crate("std::io", format!("{:?}", error))
    }
}
