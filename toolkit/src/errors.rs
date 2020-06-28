#[derive(Debug, Error)]
pub enum PrivateKeyError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}

impl From<snarkos_errors::objects::account::AccountError> for PrivateKeyError {
    fn from(error: snarkos_errors::objects::account::AccountError) -> Self {
        PrivateKeyError::Crate("snarkos_errors::objects::account", format!("{:?}", error))
    }
}

impl From<std::io::Error> for PrivateKeyError {
    fn from(error: std::io::Error) -> Self {
        PrivateKeyError::Crate("std::io", format!("{:?}", error))
    }
}

#[derive(Debug, Error)]
pub enum PublicKeyError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}

impl From<snarkos_errors::objects::account::AccountError> for PublicKeyError {
    fn from(error: snarkos_errors::objects::account::AccountError) -> Self {
        PublicKeyError::Crate("snarkos_errors::objects::account", format!("{:?}", error))
    }
}

impl From<std::io::Error> for PublicKeyError {
    fn from(error: std::io::Error) -> Self {
        PublicKeyError::Crate("std::io", format!("{:?}", error))
    }
}
