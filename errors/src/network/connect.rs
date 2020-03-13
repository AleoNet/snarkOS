use crate::network::message::{MessageError, MessageHeaderError};

use std::net::SocketAddr;

#[derive(Debug, Fail)]
pub enum ConnectError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "expected network magic prefix {}. Got {}", _0, _1)]
    InvalidMagic(u32, u32),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    MessageHeaderError(MessageHeaderError),

    #[fail(display = "{}", _0)]
    MessageError(MessageError),

    #[fail(display = "Address {:?} not found", _0)]
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
