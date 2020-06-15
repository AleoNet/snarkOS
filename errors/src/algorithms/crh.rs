use std::io::{Error, ErrorKind};

#[derive(Debug, Error)]
pub enum CRHError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("incorrect input length {} for window params {}x{}", _0, _1, _2)]
    IncorrectInputSize(usize, usize, usize),

    #[error("incorrect pp of size {}x{} for window params {}x{}", _0, _1, _2, _3)]
    IncorrectParameterSize(usize, usize, usize, usize),

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
