use std::io::{Error, ErrorKind};

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),
}

impl From<Error> for SignatureError {
    fn from(error: Error) -> Self {
        SignatureError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<SignatureError> for Error {
    fn from(error: SignatureError) -> Error {
        Error::new(ErrorKind::Other, error.to_string())
    }
}
