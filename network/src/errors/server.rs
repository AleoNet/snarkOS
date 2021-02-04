// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::errors::{message::MessageError, ConnectError, SendError};
use snarkos_storage::error::StorageError;
use snarkvm_errors::objects::{BlockError, TransactionError};

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("{}", _0)]
    BlockError(BlockError),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    MessageError(MessageError),

    #[error("{}", _0)]
    SendError(SendError),

    #[error("{}", _0)]
    StorageError(StorageError),

    #[error("{}", _0)]
    TransactionError(TransactionError),
}

impl From<BlockError> for NodeError {
    fn from(error: BlockError) -> Self {
        NodeError::BlockError(error)
    }
}

impl From<ConnectError> for NodeError {
    fn from(error: ConnectError) -> Self {
        NodeError::ConnectError(error)
    }
}

impl From<MessageError> for NodeError {
    fn from(error: MessageError) -> Self {
        NodeError::MessageError(error)
    }
}

impl From<SendError> for NodeError {
    fn from(error: SendError) -> Self {
        NodeError::SendError(error)
    }
}

impl From<StorageError> for NodeError {
    fn from(error: StorageError) -> Self {
        NodeError::StorageError(error)
    }
}

impl From<TransactionError> for NodeError {
    fn from(error: TransactionError) -> Self {
        NodeError::TransactionError(error)
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
