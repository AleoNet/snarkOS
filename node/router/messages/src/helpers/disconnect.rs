// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use serde::{Deserialize, Serialize};

/// The reason behind the node disconnecting from a peer.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
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
