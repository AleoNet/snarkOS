use std::io::{Error, ErrorKind};

#[derive(Debug, Fail)]
pub enum CRHError {
    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),
}

impl From<Error> for CRHError {
    fn from(error: Error) -> Self {
        CRHError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<CRHError> for Error {
    fn from(error: CRHError) -> Error {
        Error::new(ErrorKind::Other, error.to_string())
    }
}
