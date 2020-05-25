use crate::network::message::StreamReadError;

#[derive(Debug, Error)]
pub enum MessageHeaderError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("Invalid message header length {}. Expected length of 16", _0)]
    InvalidLength(usize),

    #[error("{}", _0)]
    StreamReadError(StreamReadError),
}

impl From<StreamReadError> for MessageHeaderError {
    fn from(error: StreamReadError) -> Self {
        MessageHeaderError::StreamReadError(error)
    }
}

impl From<bincode::Error> for MessageHeaderError {
    fn from(error: bincode::Error) -> Self {
        MessageHeaderError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<std::io::Error> for MessageHeaderError {
    fn from(error: std::io::Error) -> Self {
        MessageHeaderError::Crate("std::io", format!("{:?}", error))
    }
}
