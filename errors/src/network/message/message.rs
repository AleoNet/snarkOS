use crate::network::message::{MessageHeaderError, StreamReadError};

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("Invalid message length {}. Expected length of {}", _0, _1)]
    InvalidLength(usize, usize),

    #[error("{}", _0)]
    MessageHeaderError(MessageHeaderError),

    #[error("{}", 0)]
    SteamReadError(StreamReadError),
}

impl From<MessageHeaderError> for MessageError {
    fn from(error: MessageHeaderError) -> Self {
        MessageError::MessageHeaderError(error)
    }
}

impl From<StreamReadError> for MessageError {
    fn from(error: StreamReadError) -> Self {
        MessageError::SteamReadError(error)
    }
}

impl From<bincode::Error> for MessageError {
    fn from(error: bincode::Error) -> Self {
        MessageError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<std::io::Error> for MessageError {
    fn from(error: std::io::Error) -> Self {
        MessageError::Crate("std::io", format!("{:?}", error))
    }
}
