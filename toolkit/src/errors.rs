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
pub enum AddressError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),
}

impl From<snarkos_errors::objects::account::AccountError> for AddressError {
    fn from(error: snarkos_errors::objects::account::AccountError) -> Self {
        AddressError::Crate("snarkos_errors::objects::account", format!("{:?}", error))
    }
}

impl From<std::io::Error> for AddressError {
    fn from(error: std::io::Error) -> Self {
        AddressError::Crate("std::io", format!("{:?}", error))
    }
}
