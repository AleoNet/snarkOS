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

use crate::errors::message::{MessageError, MessageHeaderError, StreamReadError};

use std::{io::ErrorKind, net::SocketAddr};

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error(transparent)]
    Std(std::io::Error),

    #[error("{}", _0)]
    Message(String),

    #[error(transparent)]
    MessageHeaderError(MessageHeaderError),

    #[error(transparent)]
    MessageError(MessageError),

    #[error("Address {:?} not found", _0)]
    AddressNotFound(SocketAddr),
}

impl ConnectError {
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::MessageError(MessageError::StreamReadError(StreamReadError::Io(err)))
            | Self::MessageHeaderError(MessageHeaderError::StreamReadError(StreamReadError::Io(err)))
            | Self::MessageHeaderError(MessageHeaderError::Io(err))
            | Self::Std(err) => [
                ErrorKind::BrokenPipe,
                ErrorKind::ConnectionReset,
                ErrorKind::UnexpectedEof,
            ]
            .contains(&err.kind()),
            Self::MessageHeaderError(MessageHeaderError::TooBig(..))
            | Self::MessageHeaderError(MessageHeaderError::ZeroLength) => true,
            _ => false,
        }
    }
}

impl From<MessageError> for ConnectError {
    fn from(error: MessageError) -> Self {
        ConnectError::MessageError(error)
    }
}

impl From<MessageHeaderError> for ConnectError {
    fn from(error: MessageHeaderError) -> Self {
        ConnectError::MessageHeaderError(error)
    }
}

impl From<std::io::Error> for ConnectError {
    fn from(error: std::io::Error) -> Self {
        ConnectError::Std(error)
    }
}
