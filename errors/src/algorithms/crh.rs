use std::io::{Error, ErrorKind};

#[derive(Debug, Error)]
pub enum CRHError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("incorrect input length {} for window params {}x{}", _0, _1)]
    IncorrectInputSize(usize, usize),

    #[error("{}", _0)]
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
