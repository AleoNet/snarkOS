// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::errors::{message::MessageError, ConnectError, HandshakeError, PingProtocolError, SendError};
use snarkos_storage::error::StorageError;
use snarkvm_errors::objects::{BlockError, TransactionError};

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("{}", _0)]
    BlockError(BlockError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("{}", _0)]
    HandshakeError(HandshakeError),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    MessageError(MessageError),

    #[error("{}", _0)]
    PingProtocolError(PingProtocolError),

    #[error("{}", _0)]
    SendError(SendError),

    #[error("{}", _0)]
    StorageError(StorageError),

    #[error("{}", _0)]
    TransactionError(TransactionError),
}

impl From<BlockError> for ServerError {
    fn from(error: BlockError) -> Self {
        ServerError::BlockError(error)
    }
}

impl From<ConnectError> for ServerError {
    fn from(error: ConnectError) -> Self {
        ServerError::ConnectError(error)
    }
}

impl From<HandshakeError> for ServerError {
    fn from(error: HandshakeError) -> Self {
        ServerError::HandshakeError(error)
    }
}

impl From<MessageError> for ServerError {
    fn from(error: MessageError) -> Self {
        ServerError::MessageError(error)
    }
}

impl From<PingProtocolError> for ServerError {
    fn from(error: PingProtocolError) -> Self {
        ServerError::PingProtocolError(error)
    }
}

impl From<SendError> for ServerError {
    fn from(error: SendError) -> Self {
        ServerError::SendError(error)
    }
}

impl From<StorageError> for ServerError {
    fn from(error: StorageError) -> Self {
        ServerError::StorageError(error)
    }
}

impl From<TransactionError> for ServerError {
    fn from(error: TransactionError) -> Self {
        ServerError::TransactionError(error)
    }
}

impl From<std::io::Error> for ServerError {
    fn from(error: std::io::Error) -> Self {
        ServerError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<std::net::AddrParseError> for ServerError {
    fn from(error: std::net::AddrParseError) -> Self {
        ServerError::Crate("std::net::AddrParseError", format!("{:?}", error))
    }
}

impl From<bincode::Error> for ServerError {
    fn from(error: bincode::Error) -> Self {
        ServerError::Crate("bincode", format!("{:?}", error))
    }
}
