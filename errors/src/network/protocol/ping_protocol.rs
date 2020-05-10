use crate::network::{ConnectError, SendError};

use std::net::SocketAddr;

#[derive(Debug, Error)]
pub enum PingProtocolError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("Expected nonce: {}, got {}", _0, _1)]
    InvalidNonce(u64, u64),

    #[error("No stored ping for peer {:?}", _0)]
    PingProtocolMissing(SocketAddr),

    #[error("{}", _0)]
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
