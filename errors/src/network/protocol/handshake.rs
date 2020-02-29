use crate::network::{message::MessageError, ConnectError, SendError};
use std::net::SocketAddr;

#[derive(Debug, Fail)]
pub enum HandshakeError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "Peer disconnected {}", _0)]
    PeerDisconnect(SocketAddr),

    #[fail(display = "No handshake found for peer: {:?}", _0)]
    HandshakeMissing(SocketAddr),

    #[fail(display = "Handshake message expected. Got {:?}", _0)]
    InvalidMessage(String),

    #[fail(display = "Version message expected. Got {:?}", _0)]
    InvalidVersion(String),

    #[fail(display = "Verack message expected. Got {:?}", _0)]
    InvalidVerack(String),

    #[fail(display = "Expected nonce {}. Got {}", _0, _1)]
    InvalidNonce(u64, u64),

    #[fail(display = "{}", _0)]
    ConnectError(ConnectError),

    #[fail(display = "{}", _0)]
    SendError(SendError),

    #[fail(display = "{}", _0)]
    MessageError(MessageError),
}

impl From<ConnectError> for HandshakeError {
    fn from(error: ConnectError) -> Self {
        HandshakeError::ConnectError(error)
    }
}

impl From<SendError> for HandshakeError {
    fn from(error: SendError) -> Self {
        HandshakeError::SendError(error)
    }
}

impl From<MessageError> for HandshakeError {
    fn from(error: MessageError) -> Self {
        HandshakeError::MessageError(error)
    }
}
