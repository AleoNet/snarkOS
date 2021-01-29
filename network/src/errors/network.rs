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

use crate::{
    errors::{connect::ConnectError, send::SendError},
    external::Message,
};
use snarkos_consensus::error::ConsensusError;
use snarkos_storage::error::StorageError;
use snarkvm_errors::objects::BlockError;

use std::fmt;

#[derive(Debug)]
pub enum NetworkError {
    Bincode(Box<bincode::ErrorKind>),
    Bincode2(bincode::ErrorKind),
    BlockError(BlockError),
    ConnectError(ConnectError),
    ConsensusError(ConsensusError),
    IOError(std::io::Error),
    Error(anyhow::Error),
    InboundDeserializationFailed,
    InvalidHandshake,
    PeerAddressIsLocalAddress,
    PeerAlreadyConnected,
    PeerAlreadyConnecting,
    PeerAlreadyDisconnected,
    PeerAlreadyExists,
    PeerBookFailedToLoad,
    PeerBookIsCorrupt,
    PeerBookMissingPeer,
    PeerCountInvalid,
    PeerHasNeverConnected,
    PeerIsDisconnected,
    PeerIsMissingNonce,
    PeerIsReusingNonce,
    PeerNonceMismatch,
    PeerUnauthorized,
    PeerWasNotSetToConnecting,
    SelfConnectAttempt,
    SendError(SendError),
    SenderError(tokio::sync::mpsc::error::SendError<Message>),
    OutboundChannelMissing,
    OutboundPendingRequestsMissing,
    ReceiverFailedToParse,
    SendRequestUnauthorized,
    StorageError(StorageError),
    SyncIntervalInvalid,
    TryLockError(tokio::sync::TryLockError),
}

impl From<BlockError> for NetworkError {
    fn from(error: BlockError) -> Self {
        NetworkError::BlockError(error)
    }
}

impl From<ConnectError> for NetworkError {
    fn from(error: ConnectError) -> Self {
        NetworkError::ConnectError(error)
    }
}

impl From<ConsensusError> for NetworkError {
    fn from(error: ConsensusError) -> Self {
        NetworkError::ConsensusError(error)
    }
}

impl From<SendError> for NetworkError {
    fn from(error: SendError) -> Self {
        NetworkError::SendError(error)
    }
}

impl From<StorageError> for NetworkError {
    fn from(error: StorageError) -> Self {
        NetworkError::StorageError(error)
    }
}

impl From<Box<bincode::ErrorKind>> for NetworkError {
    fn from(error: Box<bincode::ErrorKind>) -> Self {
        NetworkError::Bincode(error)
    }
}

impl From<bincode::ErrorKind> for NetworkError {
    fn from(error: bincode::ErrorKind) -> Self {
        NetworkError::Bincode2(error)
    }
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(error: std::io::Error) -> Self {
        NetworkError::IOError(error)
    }
}

impl From<tokio::sync::TryLockError> for NetworkError {
    fn from(error: tokio::sync::TryLockError) -> Self {
        NetworkError::TryLockError(error)
    }
}

impl From<tokio::sync::mpsc::error::SendError<Message>> for NetworkError {
    fn from(error: tokio::sync::mpsc::error::SendError<Message>) -> Self {
        NetworkError::SenderError(error)
    }
}

impl From<anyhow::Error> for NetworkError {
    fn from(error: anyhow::Error) -> Self {
        NetworkError::Error(error)
    }
}

impl From<NetworkError> for anyhow::Error {
    fn from(error: NetworkError) -> Self {
        error!("{}", error);
        Self::msg(error.to_string())
    }
}
