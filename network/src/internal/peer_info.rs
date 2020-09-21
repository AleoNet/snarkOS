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

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::net::SocketAddr;

pub enum PeerStatus {
    NeverConnected,
    Connected,
    Disconnected,
    Unknown,
}

/// A data structure that contains metadata about a peer.
#[derive(Clone, Debug, Serialize)]
pub struct PeerInfo {
    /// The IP address of the peer.
    address: SocketAddr,
    /// The number of times we connected to the peer.
    connected_count: i64,
    /// The number of times we disconnected from the peer.
    disconnected_count: i64,
    /// The last datetime we connected to the peer.
    last_connected: DateTime<Utc>,
    /// The last datetime we disconnected from the peer.
    last_disconnected: DateTime<Utc>,
    /// The last datetime we saw the peer.
    last_seen: DateTime<Utc>,
    /// The first datetime we saw the peer.
    first_seen: DateTime<Utc>,
}

impl PeerInfo {
    /// Creates a new instance of `PeerInfo`.
    #[inline]
    pub fn new(address: SocketAddr) -> Self {
        let now = Utc::now();
        Self {
            address,
            connected_count: 0,
            disconnected_count: 0,
            last_connected: now.clone(),
            last_disconnected: now.clone(),
            last_seen: now.clone(),
            first_seen: now,
        }
    }

    /// Updates the last seen datetime of the peer,
    /// if the node is connected to the peer.
    #[inline]
    pub fn set_last_seen(&mut self) {
        // Only update the last seen datetime if the peer is connected.
        match self.status() {
            PeerStatus::Connected => {
                self.last_seen = Utc::now();
            }
            PeerStatus::Disconnected | PeerStatus::NeverConnected | PeerStatus::Unknown => {
                error!(
                    "Attempted to set the last seen datetime for a disconnected peer ({})",
                    self.address
                );
            }
        }
    }

    /// Updates the connected metrics of the peer.
    #[inline]
    pub fn set_connected(&mut self) {
        // Only update the last connected metrics if the peer is new or currently disconnected.
        match self.status() {
            PeerStatus::NeverConnected | PeerStatus::Disconnected => {
                let now = Utc::now();
                self.last_seen = now.clone();
                self.last_connected = now;
                self.connected_count += 1;
            }
            PeerStatus::Connected | PeerStatus::Unknown => {
                error!(
                    "Attempted to set a connected peer to connected again ({})",
                    self.address
                );
            }
        }
    }

    /// Updates the disconnected metrics of the peer.
    #[inline]
    pub fn set_disconnected(&mut self) {
        // Only update the last disconnected metrics if the peer is connected.
        match self.status() {
            PeerStatus::Connected => {
                let now = Utc::now();
                self.last_seen = now.clone();
                self.last_disconnected = now;
                self.disconnected_count += 1;
            }
            PeerStatus::Disconnected | PeerStatus::NeverConnected | PeerStatus::Unknown => {
                error!(
                    "Attempted to set a disconnected peer to disconnected again ({})",
                    self.address
                );
            }
        }
    }

    /// Returns the status of the peer connection based on the datetime
    /// that the node connected and disconnected to the peer.
    #[inline]
    pub fn status(&self) -> PeerStatus {
        // If `first_seen`, `last_connected`, and `last_disconnected` are all equal,
        // it means we are unnecessarily refreshing.
        if (self.first_seen == self.last_connected) && (self.last_connected == self.last_disconnected) {
            PeerStatus::NeverConnected
        }
        // If `first_seen` is earlier than `last_connected`,
        // `last_connected` is later than `last_disconnected`,
        // `last_connected` is later than the current time,
        // and `last_connected` is close to the current time,
        // it means we are connected to this peer.
        else if (self.first_seen < self.last_connected) && (self.last_connected > self.last_disconnected) {
            PeerStatus::Connected
        }
        // If `first_seen` is earlier than `last_connected`,
        // `last_connected` is earlier than `last_disconnected`,
        // `last_disconnected` is later than the current time,
        // and `last_disconnected` is close to the current time,
        // it means we are disconnected from this peer.
        else if (self.first_seen < self.last_connected) && (self.last_connected < self.last_disconnected) {
            PeerStatus::Disconnected
        }
        // If the above cases did not address our needs,
        // it is likely that either:
        //
        // 1. There is a bug in how the `PeerBook` is refreshing `PeerInfo`.
        // 2. A malicious peer or system is fudging `PeerInfo` state.
        // 3. The `PeerBook` is incorrectly triggering a refresh for an unknown reason.
        else {
            error!("The peer info of {} is refreshing incorrectly or corrupt", self.address);
            PeerStatus::Unknown
        }
    }

    /// Returns the IP address of the peer.
    #[inline]
    pub fn address(&self) -> &SocketAddr {
        &self.address
    }

    /// Returns the number of times we connected with the peer.
    #[inline]
    pub fn connected_count(&self) -> i64 {
        self.connected_count
    }

    /// Returns the number of times we disconnected with the peer.
    #[inline]
    pub fn disconnected_count(&self) -> i64 {
        self.disconnected_count
    }

    /// Returns the last datetime we connected with the peer.
    #[inline]
    pub fn last_connected(&self) -> &DateTime<Utc> {
        &self.last_connected
    }

    /// Returns the last datetime we disconnected with the peer.
    #[inline]
    pub fn last_disconnected(&self) -> &DateTime<Utc> {
        &self.last_disconnected
    }

    /// Returns the last datetime we saw the peer.
    #[inline]
    pub fn last_seen(&self) -> &DateTime<Utc> {
        &self.last_seen
    }

    /// Returns the first datetime we saw the peer.
    #[inline]
    pub fn first_seen(&self) -> &DateTime<Utc> {
        &self.first_seen
    }
}
