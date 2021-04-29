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

use std::time::Instant;

use chrono::{DateTime, Utc};
use snarkos_storage::BlockHeight;

#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct PeerQuality {
    pub block_height: BlockHeight,
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub expecting_pong: bool,
    #[serde(skip)]
    pub last_ping_sent: Option<Instant>,
    /// The time it took to send a `Ping` to the peer and for it to respond with a `Pong`.
    pub rtt_ms: u64,
    /// The number of failures associated with the peer; grounds for dismissal.
    pub failures: u32,
    /// The number of remaining blocks to sync with.
    pub remaining_sync_blocks: u32,
    pub num_messages_received: u64,
    pub first_seen: Option<DateTime<Utc>>,
    pub last_connected: Option<DateTime<Utc>>,
    pub last_disconnected: Option<DateTime<Utc>>,
    /// The number of times we have connected to this peer.
    pub connected_count: u64,
    pub disconnected_count: u64,
}

impl PeerQuality {
    pub fn is_inactive(&self, now: DateTime<Utc>) -> bool {
        let last_seen = self.last_seen;
        if let Some(last_seen) = last_seen {
            now - last_seen > chrono::Duration::seconds(crate::MAX_PEER_INACTIVITY_SECS.into())
        } else {
            // in the peer book, but never been connected to before
            false
        }
    }

    pub fn see(&mut self) {
        let now = chrono::Utc::now();
        if self.first_seen.is_none() {
            self.first_seen = Some(now);
        }
        self.last_seen = Some(now);
    }

    pub fn connected(&mut self) {
        self.see();
        self.last_connected = Some(chrono::Utc::now());
        self.connected_count += 1;
    }

    pub fn disconnected(&mut self) {
        self.see();
        self.last_disconnected = Some(chrono::Utc::now());
        self.disconnected_count += 1;
        self.expecting_pong = false;
        self.remaining_sync_blocks = 0;
    }
}
