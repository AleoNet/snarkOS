#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("Missing inverse for group element")]
    MissingInverse,

    #[error("{}", _0)]
    Message(String),
}

impl From<std::io::Error> for EncryptionError {
    fn from(error: std::io::Error) -> Self {
        EncryptionError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<EncryptionError> for std::io::Error {
    fn from(error: EncryptionError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{}", error))
    }
}
