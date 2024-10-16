// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use snarkvm::prelude::{error, FromBytes, ToBytes};

use std::io;

/// The reason behind the node disconnecting from a peer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DisconnectReason {
    /// The fork length limit was exceeded.
    ExceededForkRange,
    /// The peer's challenge response is invalid.
    InvalidChallengeResponse,
    /// The peer's client uses an invalid fork depth.
    InvalidForkDepth,
    /// The node is a sync node and the peer is ahead.
    INeedToSyncFirst,
    /// No reason given.
    NoReasonGiven,
    /// The peer is not following the protocol.
    ProtocolViolation,
    /// The peer's client is outdated, judging by its version.
    OutdatedClientVersion,
    /// Dropping a dead connection.
    PeerHasDisconnected,
    /// Dropping a connection for a periodic refresh.
    PeerRefresh,
    /// The node is shutting down.
    ShuttingDown,
    /// The sync node has served its purpose.
    SyncComplete,
    /// The peer has caused too many failures.
    TooManyFailures,
    /// The node has too many connections already.
    TooManyPeers,
    /// The peer is a sync node that's behind our node, and it needs to sync itself first.
    YouNeedToSyncFirst,
    /// The peer's listening port is closed.
    YourPortIsClosed(u16),
}

impl ToBytes for DisconnectReason {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        match self {
            Self::ExceededForkRange => 0u8.write_le(writer),
            Self::InvalidChallengeResponse => 1u8.write_le(writer),
            Self::InvalidForkDepth => 2u8.write_le(writer),
            Self::INeedToSyncFirst => 3u8.write_le(writer),
            Self::NoReasonGiven => 4u8.write_le(writer),
            Self::ProtocolViolation => 5u8.write_le(writer),
            Self::OutdatedClientVersion => 6u8.write_le(writer),
            Self::PeerHasDisconnected => 7u8.write_le(writer),
            Self::PeerRefresh => 8u8.write_le(writer),
            Self::ShuttingDown => 9u8.write_le(writer),
            Self::SyncComplete => 10u8.write_le(writer),
            Self::TooManyFailures => 11u8.write_le(writer),
            Self::TooManyPeers => 12u8.write_le(writer),
            Self::YouNeedToSyncFirst => 13u8.write_le(writer),
            Self::YourPortIsClosed(port) => {
                14u8.write_le(&mut writer)?;
                port.write_le(writer)
            }
        }
    }
}

impl FromBytes for DisconnectReason {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        match u8::read_le(&mut reader)? {
            0 => Ok(Self::ExceededForkRange),
            1 => Ok(Self::InvalidChallengeResponse),
            2 => Ok(Self::InvalidForkDepth),
            3 => Ok(Self::INeedToSyncFirst),
            4 => Ok(Self::NoReasonGiven),
            5 => Ok(Self::ProtocolViolation),
            6 => Ok(Self::OutdatedClientVersion),
            7 => Ok(Self::PeerHasDisconnected),
            8 => Ok(Self::PeerRefresh),
            9 => Ok(Self::ShuttingDown),
            10 => Ok(Self::SyncComplete),
            11 => Ok(Self::TooManyFailures),
            12 => Ok(Self::TooManyPeers),
            13 => Ok(Self::YouNeedToSyncFirst),
            14 => {
                let port = u16::read_le(reader)?;
                Ok(Self::YourPortIsClosed(port))
            }
            _ => Err(error("Invalid disconnect reason")),
        }
    }
}
