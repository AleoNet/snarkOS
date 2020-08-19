use crate::{
    algorithms::CRHError,
    consensus::ConsensusError,
    network::ServerError,
    node::CliError,
    objects::AccountError,
    storage::StorageError,
};

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("{}", _0)]
    AccountError(AccountError),

    #[error("{}", _0)]
    CLIError(CliError),

    #[error("{}", _0)]
    CRHError(CRHError),

    #[error("{}", _0)]
    ConsensusError(ConsensusError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    ServerError(ServerError),

    #[error("{}", _0)]
    StorageError(StorageError),
}

impl From<AccountError> for NodeError {
    fn from(error: AccountError) -> Self {
        NodeError::AccountError(error)
    }
}

impl From<CliError> for NodeError {
    fn from(error: CliError) -> Self {
        NodeError::CLIError(error)
    }
}

impl From<CRHError> for NodeError {
    fn from(error: CRHError) -> Self {
        NodeError::CRHError(error)
    }
}

impl From<ConsensusError> for NodeError {
    fn from(error: ConsensusError) -> Self {
        NodeError::ConsensusError(error)
    }
}

impl From<hex::FromHexError> for NodeError {
    fn from(error: hex::FromHexError) -> Self {
        NodeError::Crate("hex", format!("{:?}", error))
    }
}

impl From<ServerError> for NodeError {
    fn from(error: ServerError) -> Self {
        NodeError::ServerError(error)
    }
}

impl From<StorageError> for NodeError {
    fn from(error: StorageError) -> Self {
        NodeError::StorageError(error)
    }
}

impl From<std::io::Error> for NodeError {
    fn from(error: std::io::Error) -> Self {
        NodeError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<std::net::AddrParseError> for NodeError {
    fn from(error: std::net::AddrParseError) -> Self {
        NodeError::Crate("std::net::AddrParseError", format!("{:?}", error))
    }
}

impl From<std::boxed::Box<dyn std::error::Error>> for NodeError {
    fn from(error: std::boxed::Box<dyn std::error::Error>) -> Self {
        NodeError::Crate("std::boxed::Box", format!("{:?}", error))
    }
}
