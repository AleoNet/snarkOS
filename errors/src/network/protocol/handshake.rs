use crate::network::{message::MessageError, ConnectError, SendError};
use std::net::SocketAddr;

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("Peer disconnected {}", _0)]
    PeerDisconnect(SocketAddr),

    #[error("No handshake found for peer: {:?}", _0)]
    HandshakeMissing(SocketAddr),

    #[error("Handshake message expected. Got {:?}", _0)]
    InvalidMessage(String),

    #[error("Version message expected. Got {:?}", _0)]
    InvalidVersion(String),

    #[error("Verack message expected. Got {:?}", _0)]
    InvalidVerack(String),

    #[error("Expected nonce {}. Got {}", _0, _1)]
    InvalidNonce(u64, u64),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("{}", _0)]
    SendError(SendError),

    #[error("{}", _0)]
    MessageError(MessageError),
}

impl From<ConnectError> for HandshakeError {
    fn from(error: ConnectError) -> Self {
        HandshakeError::ConnectError(error)
    }
}

impl From<MessageError> for HandshakeError {
    fn from(error: MessageError) -> Self {
        HandshakeError::MessageError(error)
    }
}

impl From<SendError> for HandshakeError {
    fn from(error: SendError) -> Self {
        HandshakeError::SendError(error)
    }
}
