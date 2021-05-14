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

use crate::NetworkError;
use snarkos_storage::BlockHeight;

use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use tokio::task;

use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum PeerStatus {
    Connected,
    Disconnected,
    NeverConnected,
}

#[derive(Debug, Default)]
pub struct PeerQuality {
    /// The current block height of this peer.
    pub block_height: AtomicU32,
    /// The timestamp of when the peer has been seen last.
    pub last_seen: RwLock<Option<DateTime<Utc>>>,
    /// An indicator of whether a `Pong` message is currently expected from this peer.
    pub expecting_pong: AtomicBool,
    /// The timestamp of the last `Ping` sent to the peer.
    pub last_ping_sent: Mutex<Option<Instant>>,
    /// The time it took to send a `Ping` to the peer and for it to respond with a `Pong`.
    pub rtt_ms: AtomicU64,
    /// The number of failures associated with the peer; grounds for dismissal.
    pub failures: AtomicU32,
    /// The number of remaining blocks to sync with.
    pub remaining_sync_blocks: AtomicU32,
}

impl PeerQuality {
    pub fn is_inactive(&self, now: DateTime<Utc>) -> bool {
        let last_seen = *self.last_seen.read();
        if let Some(last_seen) = last_seen {
            now - last_seen > chrono::Duration::seconds(crate::MAX_PEER_INACTIVITY_SECS.into())
        } else {
            // Impossible to trigger, as completing a connection
            // marks the peer with a timestamp. That being said,
            // it's safest to leave this as `true` for future-proofing.
            true
        }
    }
}

/// A data structure containing information about a peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// The IP address of this peer.
    address: SocketAddr,
    /// The current status of this peer.
    status: PeerStatus,
    /// The timestamp of the first seen instance of this peer.
    first_seen: Option<DateTime<Utc>>,
    /// The timestamp of the last seen instance of this peer.
    last_connected: Option<DateTime<Utc>>,
    /// The timestamp of the last disconnect from this peer.
    last_disconnected: Option<DateTime<Utc>>,
    /// The number of times we have connected to this peer.
    connected_count: u64,
    /// The number of times we have disconnected from this peer.
    disconnected_count: u64,
    /// The quality of the connection with the peer.
    #[serde(skip)]
    pub quality: Arc<PeerQuality>,
    /// The handles for tasks associated exclusively with this peer;
    /// The bool indicates whether it's abortable - otherwise it
    /// needs to be awaited instead.
    #[serde(skip)]
    #[allow(clippy::type_complexity)]
    pub tasks: Arc<Mutex<Vec<(task::JoinHandle<()>, bool)>>>,
}

impl PeerInfo {
    ///
    /// Creates a new instance of `PeerInfo`.
    ///
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            status: PeerStatus::NeverConnected,
            first_seen: None,
            last_connected: None,
            last_disconnected: None,
            connected_count: 0,
            disconnected_count: 0,
            quality: Default::default(),
            tasks: Default::default(),
        }
    }

    ///
    /// Returns the IP address of this peer.
    ///
    #[inline]
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    ///
    /// Returns the current status of this peer.
    ///
    #[inline]
    pub fn status(&self) -> PeerStatus {
        self.status
    }

    ///
    /// Returns the current block height of this peer.
    ///
    #[inline]
    pub fn block_height(&self) -> BlockHeight {
        self.quality.block_height.load(Ordering::SeqCst)
    }

    ///
    /// Returns the timestamp of the first seen instance of this peer.
    ///
    #[inline]
    pub fn first_seen(&self) -> Option<DateTime<Utc>> {
        self.first_seen
    }

    ///
    /// Returns the timestamp of the last seen instance of this peer.
    ///
    #[inline]
    pub fn last_seen(&self) -> Option<DateTime<Utc>> {
        *self.quality.last_seen.read()
    }

    ///
    /// Returns the timestamp of the last connection to this peer.
    ///
    #[inline]
    pub fn last_connected(&self) -> Option<DateTime<Utc>> {
        self.last_connected
    }

    ///
    /// Returns the timestamp of the last disconnect from this peer.
    ///
    #[inline]
    pub fn last_disconnected(&self) -> Option<DateTime<Utc>> {
        self.last_disconnected
    }

    ///
    /// Returns the number of times we have connected to this peer.
    ///
    #[inline]
    pub fn connected_count(&self) -> u64 {
        self.connected_count
    }

    ///
    /// Returns the number of times we have disconnected from this peer.
    ///
    #[inline]
    pub fn disconnected_count(&self) -> u64 {
        self.disconnected_count
    }

    ///
    /// Updates the peer to connected.
    ///
    pub(crate) fn set_connected(&mut self) {
        if self.status() != PeerStatus::Connected {
            // Set the state of this peer to connected.
            self.status = PeerStatus::Connected;

            let now = Utc::now();
            self.last_connected = Some(now);
            *self.quality.last_seen.write() = Some(now);
            self.connected_count += 1;
        } else {
            trace!("Peer {} was set as connected more than once", self.address);
        }
    }

    ///
    /// Updates the peer to disconnected.
    ///
    /// If the peer is not transitioning from `PeerStatus::Connecting` or `PeerStatus::Connected`,
    /// this function returns a `NetworkError`.
    ///
    pub(crate) fn set_disconnected(&mut self) -> Result<(), NetworkError> {
        match self.status() {
            PeerStatus::Connected => {
                // Set the state of this peer to disconnected.
                self.status = PeerStatus::Disconnected;

                self.last_disconnected = Some(Utc::now());
                self.quality.expecting_pong.store(false, Ordering::SeqCst);
                self.quality.remaining_sync_blocks.store(0, Ordering::SeqCst);
                self.disconnected_count += 1;

                for (handle, abortable) in self.tasks.lock().drain(..).rev() {
                    if abortable {
                        handle.abort();
                    } else {
                        task::spawn(async move {
                            // An arbitrary amount of time to allow the task to shut down cleanly.
                            if tokio::time::timeout(Duration::from_secs(5), handle).await.is_err() {
                                warn!("One of the per-connection tasks didn't shut down cleanly");
                            }
                        });
                    }
                }

                Ok(())
            }
            PeerStatus::Disconnected | PeerStatus::NeverConnected => {
                error!("Attempting to disconnect from a disconnected peer - {}", self.address);
                Err(NetworkError::PeerAlreadyDisconnected)
            }
        }
    }

    pub(crate) fn register_task(&self, handle: task::JoinHandle<()>, abortable: bool) {
        self.tasks.lock().push((handle, abortable));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let address: SocketAddr = "127.0.0.1:4130".parse().unwrap();
        let peer_info = PeerInfo::new(address);

        assert_eq!(address, peer_info.address());
        assert_eq!(PeerStatus::NeverConnected, peer_info.status());
        assert_eq!(0, peer_info.connected_count());
        assert_eq!(0, peer_info.disconnected_count());
    }

    #[test]
    fn test_set_disconnected_from_connected() {
        let address: SocketAddr = "127.0.0.1:4130".parse().unwrap();
        let mut peer_info = PeerInfo::new(address);

        peer_info.set_connected();
        assert_eq!(address, peer_info.address());
        assert_eq!(PeerStatus::Connected, peer_info.status());
        assert_eq!(1, peer_info.connected_count());
        assert_eq!(0, peer_info.disconnected_count());

        peer_info.set_disconnected().unwrap();
        assert_eq!(address, peer_info.address());
        assert_eq!(PeerStatus::Disconnected, peer_info.status());
        assert_eq!(1, peer_info.connected_count());
        assert_eq!(1, peer_info.disconnected_count());
    }

    #[test]
    fn test_set_disconnected_from_never_connected() {
        let address: SocketAddr = "127.0.0.1:4130".parse().unwrap();
        let mut peer_info = PeerInfo::new(address);

        assert!(peer_info.set_disconnected().is_err());

        assert_eq!(address, peer_info.address());
        assert_eq!(PeerStatus::NeverConnected, peer_info.status());
        assert_eq!(0, peer_info.connected_count());
        assert_eq!(0, peer_info.disconnected_count());
    }

    #[test]
    fn test_set_connected_from_disconnected() {
        let address: SocketAddr = "127.0.0.1:4130".parse().unwrap();
        let mut peer_info = PeerInfo::new(address);

        peer_info.set_connected();
        peer_info.set_disconnected().unwrap();
        assert_eq!(address, peer_info.address());
        assert_eq!(PeerStatus::Disconnected, peer_info.status());
        assert_eq!(1, peer_info.connected_count());
        assert_eq!(1, peer_info.disconnected_count());

        peer_info.set_connected();

        assert_eq!(address, peer_info.address());
        assert_eq!(PeerStatus::Connected, peer_info.status());
        assert_eq!(2, peer_info.connected_count());
        assert_eq!(1, peer_info.disconnected_count());
    }
}
