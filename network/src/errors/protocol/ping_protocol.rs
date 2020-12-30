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

use crate::network::{ConnectError, SendError};

use std::net::SocketAddr;

#[derive(Debug, Error)]
pub enum PingProtocolError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("Expected nonce: {}, got {}", _0, _1)]
    InvalidNonce(u64, u64),

    #[error("No stored ping for peer {:?}", _0)]
    PingProtocolMissing(SocketAddr),

    #[error("{}", _0)]
    SendError(SendError),
}

impl From<ConnectError> for PingProtocolError {
    fn from(error: ConnectError) -> Self {
        PingProtocolError::ConnectError(error)
    }
}

impl From<SendError> for PingProtocolError {
    fn from(error: SendError) -> Self {
        PingProtocolError::SendError(error)
    }
}
