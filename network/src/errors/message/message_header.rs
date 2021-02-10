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

use crate::errors::message::StreamReadError;

#[derive(Debug, Error)]
pub enum MessageHeaderError {
    #[error("IO error: {}", _0)]
    Io(std::io::Error),

    #[error("{}", _0)]
    Message(String),

    #[error("The message is too big ({}B). Maximum size: {}", _0, _1)]
    TooBig(usize, usize),

    #[error("{}", _0)]
    StreamReadError(StreamReadError),

    #[error("Zero-sized message")]
    ZeroLength,
}

impl From<StreamReadError> for MessageHeaderError {
    fn from(error: StreamReadError) -> Self {
        MessageHeaderError::StreamReadError(error)
    }
}

impl From<std::io::Error> for MessageHeaderError {
    fn from(error: std::io::Error) -> Self {
        MessageHeaderError::Io(error)
    }
}
