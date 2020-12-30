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

use crate::{
    consensus::ConsensusError,
    network::{message::MessageError, ConnectError},
    objects::BlockError,
};

#[derive(Debug, Error)]
pub enum SendError {
    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    ConnectError(ConnectError),

    #[error("{}", _0)]
    ConsensusError(ConsensusError),

    #[error("{}", _0)]
    Message(String),

    #[error("{}", _0)]
    MessageError(MessageError),

    #[error("{}", _0)]
    BlockError(BlockError),
}

impl From<BlockError> for SendError {
    fn from(error: BlockError) -> Self {
        SendError::BlockError(error)
    }
}

impl From<ConnectError> for SendError {
    fn from(error: ConnectError) -> Self {
        SendError::ConnectError(error)
    }
}

impl From<ConsensusError> for SendError {
    fn from(error: ConsensusError) -> Self {
        SendError::ConsensusError(error)
    }
}

impl From<MessageError> for SendError {
    fn from(error: MessageError) -> Self {
        SendError::MessageError(error)
    }
}
