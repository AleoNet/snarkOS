use crate::network::message::{MessageHeaderError, StreamReadError};

#[derive(Debug, Fail)]
pub enum MessageError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "Invalid message length {}. Expected length of {}", _0, _1)]
    InvalidLength(usize, usize),

    #[fail(display = "{}", _0)]
    MessageHeaderError(MessageHeaderError),

    #[fail(display = "{}", 0)]
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
