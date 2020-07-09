use std::io::{Error, ErrorKind};

#[derive(Debug, Error)]
pub enum EncodingError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),
}

impl From<Error> for EncodingError {
    fn from(error: Error) -> Self {
        EncodingError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<EncodingError> for Error {
    fn from(error: EncodingError) -> Error {
        Error::new(ErrorKind::Other, error.to_string())
    }
}
