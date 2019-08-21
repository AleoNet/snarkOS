use crate::{consensus::ConsensusError, network::ServerError, node::CliError, storage::StorageError};

#[derive(Debug, Fail)]
pub enum NodeError {
    #[fail(display = "{}", _0)]
    CLIError(CliError),

    #[fail(display = "{}", _0)]
    ConsensusError(ConsensusError),

    #[fail(display = "{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[fail(display = "{}", _0)]
    Message(String),

    #[fail(display = "{}", _0)]
    ServerError(ServerError),

    #[fail(display = "{}", _0)]
    StorageError(StorageError),
}

impl From<CliError> for NodeError {
    fn from(error: CliError) -> Self {
        NodeError::CLIError(error)
    }
}

impl From<ConsensusError> for NodeError {
    fn from(error: ConsensusError) -> Self {
        NodeError::ConsensusError(error)
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
