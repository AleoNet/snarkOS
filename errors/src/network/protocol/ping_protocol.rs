use crate::network::{ConnectError, SendError};
use std::net::SocketAddr;

#[derive(Debug, Fail)]
pub enum PingProtocolError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    ConnectError(ConnectError),

    #[fail(display = "Expected nonce: {}, got {}", _0, _1)]
    InvalidNonce(u64, u64),

    #[fail(display = "No stored ping for peer {:?}", _0)]
    PingProtocolMissing(SocketAddr),

    #[fail(display = "{}", _0)]
    SendError(SendError),
}

impl From<ConnectError> for PingProtocolError {
    fn from(error: ConnectError) -> Self {
        PingProtocolError::ConnectError(error)
    }
}

impl From<SendError> for PingProtocolError {
    fn from(error: SendError) -> Self {
        PingProtocolError::SendError(error)
    }
}
