// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use serde::{Deserialize, Serialize};

/// The reason behind the node disconnecting from a peer.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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
