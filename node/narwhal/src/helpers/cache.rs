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

use snarkvm::{console::types::Field, ledger::narwhal::TransmissionID, prelude::Network};

use core::hash::Hash;
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{
    collections::VecDeque,
    net::{IpAddr, SocketAddr},
};
use time::{Duration, OffsetDateTime};

#[derive(Debug)]
pub struct Cache<N: Network> {
    /// The map of peer connections to their recent timestamps.
    seen_inbound_connections: RwLock<IndexMap<IpAddr, VecDeque<OffsetDateTime>>>,
    /// The map of peer IPs to their recent timestamps.
    seen_inbound_events: RwLock<IndexMap<SocketAddr, VecDeque<OffsetDateTime>>>,
    /// The map of certificate IDs to their last seen timestamp.
    seen_inbound_certificates: RwLock<IndexMap<Field<N>, VecDeque<OffsetDateTime>>>,
    /// The map of transmission IDs to their last seen timestamp.
    seen_inbound_transmissions: RwLock<IndexMap<TransmissionID<N>, VecDeque<OffsetDateTime>>>,
    /// The map of peer IPs to their recent timestamps.
    seen_outbound_events: RwLock<IndexMap<SocketAddr, VecDeque<OffsetDateTime>>>,
    /// The map of peer IPs to the number of certificate requests.
    seen_outbound_certificates: RwLock<IndexMap<SocketAddr, VecDeque<OffsetDateTime>>>,
    /// The map of peer IPs to the number of transmission requests.
    seen_outbound_transmissions: RwLock<IndexMap<SocketAddr, VecDeque<OffsetDateTime>>>,
}

impl<N: Network> Default for Cache<N> {
    /// Initializes a new instance of the cache.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Cache<N> {
    /// Initializes a new instance of the cache.
    pub fn new() -> Self {
        Self {
            seen_inbound_connections: Default::default(),
            seen_inbound_events: Default::default(),
            seen_inbound_certificates: Default::default(),
            seen_inbound_transmissions: Default::default(),
            seen_outbound_events: Default::default(),
            seen_outbound_certificates: Default::default(),
            seen_outbound_transmissions: Default::default(),
        }
    }
}

impl<N: Network> Cache<N> {
    /// Inserts a new timestamp for the given peer connection, returning the number of recent connection requests.
    pub fn insert_inbound_connection(&self, peer_ip: IpAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_inbound_connections, peer_ip, interval_in_secs)
    }

    /// Inserts a new timestamp for the given peer, returning the number of recent events.
    pub fn insert_inbound_event(&self, peer_ip: SocketAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_inbound_events, peer_ip, interval_in_secs)
    }

    /// Inserts a certificate ID into the cache, returning the number of recent events.
    pub fn insert_inbound_certificate(&self, key: Field<N>, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_inbound_certificates, key, interval_in_secs)
    }

    /// Inserts a transmission ID into the cache, returning the number of recent events.
    pub fn insert_inbound_transmission(&self, key: TransmissionID<N>, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_inbound_transmissions, key, interval_in_secs)
    }
}

impl<N: Network> Cache<N> {
    /// Inserts a new timestamp for the given peer, returning the number of recent events.
    pub fn insert_outbound_event(&self, peer_ip: SocketAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_outbound_events, peer_ip, interval_in_secs)
    }

    /// Inserts a new timestamp for the given peer, returning the number of recent events.
    pub fn insert_outbound_certificate(&self, peer_ip: SocketAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_outbound_certificates, peer_ip, interval_in_secs)
    }

    /// Inserts a new timestamp for the given peer, returning the number of recent events.
    pub fn insert_outbound_transmission(&self, peer_ip: SocketAddr, interval_in_secs: i64) -> usize {
        Self::retain_and_insert(&self.seen_outbound_transmissions, peer_ip, interval_in_secs)
    }
}

impl<N: Network> Cache<N> {
    /// Insert a new timestamp for the given key, returning the number of recent entries.
    fn retain_and_insert<K: Copy + Clone + PartialEq + Eq + Hash>(
        map: &RwLock<IndexMap<K, VecDeque<OffsetDateTime>>>,
        key: K,
        interval_in_secs: i64,
    ) -> usize {
        // Fetch the current timestamp.
        let now = OffsetDateTime::now_utc();

        let mut map_write = map.write();
        // Load the entry for the key.
        let timestamps = map_write.entry(key).or_default();
        // Insert the new timestamp.
        timestamps.push_back(now);
        // Retain only the timestamps that are within the recent interval.
        while timestamps.front().map_or(false, |t| now - *t > Duration::seconds(interval_in_secs)) {
            timestamps.pop_front();
        }
        // Return the frequency of recent requests.
        timestamps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::prelude::Testnet3;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_inbound_certificate() {
        let cache = Cache::<CurrentNetwork>::default();
        let certificate_id = Field::<CurrentNetwork>::from_u8(1);

        // Check that the cache is empty.
        assert_eq!(cache.seen_inbound_certificates.read().len(), 0);

        // Insert a solution.
        assert_eq!(cache.insert_inbound_certificate(certificate_id, 5), 1);

        // Check that the cache contains the solution.
        assert_eq!(cache.seen_inbound_certificates.read().len(), 1);

        // Insert the same solution again.
        assert_eq!(cache.insert_inbound_certificate(certificate_id, 5), 2);

        // Check that the cache still contains the solution.
        assert_eq!(cache.seen_inbound_certificates.read().len(), 1);
    }

    #[test]
    fn test_inbound_transmission() {
        let cache = Cache::<CurrentNetwork>::default();
        let transmission = TransmissionID::Transaction(Default::default());

        // Check that the cache is empty.
        assert_eq!(cache.seen_inbound_transmissions.read().len(), 0);

        // Insert a transmission.
        assert_eq!(cache.insert_inbound_transmission(transmission, 5), 1);

        // Check that the cache contains the transmission.
        assert_eq!(cache.seen_inbound_transmissions.read().len(), 1);

        // Insert the same transmission again.
        assert_eq!(cache.insert_inbound_transmission(transmission, 5), 2);

        // Check that the cache still contains the transmission.
        assert_eq!(cache.seen_inbound_transmissions.read().len(), 1);
    }
}
