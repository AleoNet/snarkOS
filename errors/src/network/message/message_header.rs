use crate::network::message::StreamReadError;

#[derive(Debug, Fail)]
pub enum MessageHeaderError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "Invalid message header length {}. Expected length of 16", _0)]
    InvalidLength(usize),

    #[fail(display = "{}", _0)]
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
