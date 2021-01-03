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

use crate::errors::message::StreamReadError;

#[derive(Debug, Error)]
pub enum MessageHeaderError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("Invalid message header length {}. Expected length of 16", _0)]
    InvalidLength(usize),

    #[error("{}", _0)]
    StreamReadError(StreamReadError),
}

impl From<StreamReadError> for MessageHeaderError {
    fn from(error: StreamReadError) -> Self {
        MessageHeaderError::StreamReadError(error)
    }
}

impl From<bincode::Error> for MessageHeaderError {
    fn from(error: bincode::Error) -> Self {
        MessageHeaderError::Crate("bincode", format!("{:?}", error))
    }
}

impl From<std::io::Error> for MessageHeaderError {
    fn from(error: std::io::Error) -> Self {
        MessageHeaderError::Crate("std::io", format!("{:?}", error))
    }
}
