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

    use std::net::Ipv4Addr;

    type CurrentNetwork = Testnet3;

    trait Input {
        fn input() -> Self;
    }

    impl Input for IpAddr {
        fn input() -> Self {
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        }
    }

    impl Input for SocketAddr {
        fn input() -> Self {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 1234)
        }
    }

    impl Input for Field<CurrentNetwork> {
        fn input() -> Self {
            Field::from_u8(1)
        }
    }

    impl Input for TransmissionID<CurrentNetwork> {
        fn input() -> Self {
            TransmissionID::Transaction(Default::default())
        }
    }

    const INTERVAL_IN_SECS: i64 = 1;

    macro_rules! test_cache_fields {
        ($($name:ident),*) => {
            $(
                paste::paste! {
                    #[test]
                    fn [<test_seen_ $name s>]() {
                        let cache = Cache::<CurrentNetwork>::default();
                        let input = Input::input();

                        // Check that the cache is empty.
                        assert!(cache.[<seen_ $name s>].read().is_empty());

                        // Insert an input, recent events should be 1.
                        assert_eq!(cache.[<insert_ $name>](input, INTERVAL_IN_SECS), 1);
                        // Insert an input, recent events should be 2.
                        assert_eq!(cache.[<insert_ $name>](input, INTERVAL_IN_SECS), 2);
                        // Insert an input, recent events should be 3.
                        assert_eq!(cache.[<insert_ $name>](input, INTERVAL_IN_SECS), 3);

                        // Check that the cache contains the input for 3 entries.
                        assert_eq!(cache.[<seen_ $name s>].read().get(&input).unwrap().len(), 3);

                        // Wait for the input to expire.
                        std::thread::sleep(std::time::Duration::from_secs(INTERVAL_IN_SECS as u64 + 1));

                        // Insert an input again, recent events should be 1.
                        assert_eq!(cache.[<insert_ $name>](input, INTERVAL_IN_SECS), 1);

                        // Check that the cache still contains the input.
                        assert_eq!(cache.[<seen_ $name s>].read().len(), 1);

                        // Check that the cache contains the input and 1 timestamp entry.
                        assert_eq!(cache.[<seen_ $name s>].read().get(&input).unwrap().len(), 1);
                    }
                }
            )*
        }
    }

    test_cache_fields! {
       inbound_connection,
       inbound_event,
       inbound_certificate,
       inbound_transmission,
       outbound_event,
       outbound_certificate,
       outbound_transmission
    }
}
