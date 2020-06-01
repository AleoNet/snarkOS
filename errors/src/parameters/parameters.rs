use std::fmt::Debug;

#[derive(Debug, Error)]
pub enum ParametersError {
    #[error("expected checksum of {}, found checksum of {}", _0, _1)]
    ChecksumMismatch(String, String),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),
}

impl From<curl::Error> for ParametersError {
    fn from(error: curl::Error) -> Self {
        ParametersError::Crate("curl::error", format!("{:?}", error))
    }
}

impl From<std::io::Error> for ParametersError {
    fn from(error: std::io::Error) -> Self {
        ParametersError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<std::path::StripPrefixError> for ParametersError {
    fn from(error: std::path::StripPrefixError) -> Self {
        ParametersError::Crate("std::path", format!("{:?}", error))
    }
}

impl From<ParametersError> for std::io::Error {
    fn from(error: ParametersError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
    }
}
