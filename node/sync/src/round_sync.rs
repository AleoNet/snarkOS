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

use crate::locators::RoundLocators;
use snarkvm::console::{network::Network, types::Field};

use anyhow::Result;
use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};

#[derive(Clone, Default)]
pub struct RoundSync<N: Network> {
    /// The map of rounds to certificate IDs.
    round_to_certificate_ids: Arc<RwLock<BTreeMap<u64, IndexSet<Field<N>>>>>,
    /// The map of certificate IDs to rounds.
    certificate_id_to_round: Arc<RwLock<IndexMap<Field<N>, u64>>>,
    /// The map of certificate IDs to IPs.
    certificate_id_to_ip: Arc<RwLock<IndexMap<Field<N>, IndexSet<SocketAddr>>>>,
    /// Tracks the internal GC round (inclusive).
    gc_round: Arc<AtomicU64>,
}

impl<N: Network> RoundSync<N> {
    /// Initializes a new `RoundSync` instance.
    pub fn new() -> Self {
        Self {
            round_to_certificate_ids: Default::default(),
            certificate_id_to_round: Default::default(),
            certificate_id_to_ip: Default::default(),
            gc_round: Default::default(),
        }
    }
}

impl<N: Network> RoundSync<N> {
    /// Updates the round sync with the given IP and round locators.
    pub fn update_round_locators(&self, ip: SocketAddr, round_locators: &RoundLocators<N>) -> Result<()> {
        // Ensure the round locators are well-formed.
        round_locators.ensure_is_well_formed()?;

        // Acquire the write locks.
        let mut round_to_certificate_ids = self.round_to_certificate_ids.write();
        let mut certificate_id_to_round = self.certificate_id_to_round.write();
        let mut certificate_id_to_ip = self.certificate_id_to_ip.write();

        // Load the current GC round.
        let current_gc_round = self.gc_round.load(std::sync::atomic::Ordering::SeqCst);

        // Iterate over the round locators.
        for (round, certificate_ids) in round_locators.certificate_ids() {
            // If the round is at or below the current GC round, skip it.
            if *round <= current_gc_round {
                continue;
            }
            // Iterate over the certificate IDs.
            for certificate_id in certificate_ids {
                // Insert the certificate ID into the map.
                round_to_certificate_ids.entry(*round).or_default().insert(*certificate_id);
                certificate_id_to_round.insert(*certificate_id, *round);
                certificate_id_to_ip.entry(*certificate_id).or_default().insert(ip);
            }
        }
        Ok(())
    }

    /// Returns the missing certificate IDs (and corresponding rounds) from the round sync,
    /// for the given round locators.
    ///
    /// The returned missing certificate IDs are ones that have an overlap of at least `threshold` IPs.
    pub fn find_missing_certificates(
        &self,
        round_locators: &RoundLocators<N>,
        next_gc_round: u64,
        threshold_ips: usize,
    ) -> BTreeMap<u64, IndexMap<Field<N>, IndexSet<SocketAddr>>> {
        // Start by performing GC.
        self.gc(next_gc_round);

        // Acquire the read locks.
        let round_to_certificate_ids = self.round_to_certificate_ids.read();
        let certificate_id_to_ip = self.certificate_id_to_ip.read();

        // Iterate over the round sync.
        let mut missing_certificate_ids = BTreeMap::<u64, IndexMap<_, _>>::new();

        for (round, certificate_ids) in round_to_certificate_ids.iter() {
            for certificate_id in certificate_ids.iter() {
                // Check if the certificate ID is missing from the round locators.
                if !round_locators.contains_certificate_id(*round, *certificate_id) {
                    // Retrieve the IPs for the certificate ID.
                    if let Some(ips) = certificate_id_to_ip.get(certificate_id) {
                        // Check if the number of IPs is at least the threshold.
                        if ips.len() >= threshold_ips {
                            // Insert the certificate ID into the map.
                            missing_certificate_ids.entry(*round).or_default().insert(*certificate_id, ips.clone());
                        }
                    }
                }
            }
        }
        // Return the missing certificate IDs.
        missing_certificate_ids
    }

    /// Performs GC on the round sync, with the given GC round (inclusive).
    fn gc(&self, next_gc_round: u64) {
        // Load the current GC round.
        let current_gc_round = self.gc_round.load(std::sync::atomic::Ordering::SeqCst);
        // Check if the given GC round is greater than the current GC round.
        if next_gc_round <= current_gc_round {
            return;
        }

        // Acquire the write locks.
        let mut round_to_certificate_ids = self.round_to_certificate_ids.write();
        let mut certificate_id_to_round = self.certificate_id_to_round.write();
        let mut certificate_id_to_ip = self.certificate_id_to_ip.write();

        // Remove the rounds that are at or below the given GC round.
        let rounds = round_to_certificate_ids.keys().copied().collect::<Vec<_>>();
        for round in rounds {
            if round <= next_gc_round {
                // Remove the certificate IDs that are less than the given round.
                if let Some(mut certificate_ids) = round_to_certificate_ids.remove(&round) {
                    for certificate_id in certificate_ids.drain(..) {
                        certificate_id_to_round.remove(&certificate_id);
                        certificate_id_to_ip.remove(&certificate_id);
                    }
                }
            }
        }
        // Update the GC round.
        self.gc_round.store(next_gc_round, std::sync::atomic::Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type CurrentNetwork = snarkvm::console::network::Testnet3;

    /// Returns a sample round locators.
    fn sample_round_locators() -> RoundLocators<CurrentNetwork> {
        RoundLocators::new(
            vec![
                (1, [1, 2, 3].map(Field::<CurrentNetwork>::from_u8)),
                (2, [4, 5, 6].map(Field::<CurrentNetwork>::from_u8)),
                (3, [7, 8, 9].map(Field::<CurrentNetwork>::from_u8)),
            ]
            .into_iter()
            .map(|(round, certificate_ids)| (round, certificate_ids.into_iter().collect::<IndexSet<_>>()))
            .collect::<BTreeMap<_, _>>(),
        )
    }

    #[test]
    fn test_round_sync() {
        // Initialize the round sync.
        let round_sync = RoundSync::<CurrentNetwork>::new();

        let ip = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let round_locators = sample_round_locators();

        // Update the round sync.
        round_sync.update_round_locators(ip, &round_locators).unwrap();

        // Get the missing certificate IDs (nothing missing).
        let gc_round = 0;
        let threshold_ips = 0;
        let missing_certificate_ids = round_sync.find_missing_certificates(&round_locators, gc_round, threshold_ips);
        assert_eq!(missing_certificate_ids, Default::default());

        // Get the missing certificate IDs (all missing).
        let empty_round_locators = RoundLocators::<CurrentNetwork>::new(BTreeMap::new());
        let gc_round = 0;
        let threshold_ips = 0;
        let missing_certificate_ids =
            round_sync.find_missing_certificates(&empty_round_locators, gc_round, threshold_ips);

        // Ensure the missing certificate IDs are correct.
        let candidate = missing_certificate_ids
            .into_iter()
            .map(|(round, certificate_ids)| {
                (
                    round,
                    certificate_ids
                        .into_iter()
                        .map(|(certificate_id, ips)| {
                            assert_eq!(ips, [ip].iter().copied().collect::<IndexSet<_>>());
                            certificate_id
                        })
                        .collect::<IndexSet<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(candidate, round_locators.certificate_ids().clone());
    }

    #[test]
    fn test_round_sync_gc() {
        // Initialize the round sync.
        let round_sync = RoundSync::<CurrentNetwork>::new();

        let ip = "127.0.0.1:0".parse::<SocketAddr>().unwrap();
        let round_locators = sample_round_locators();

        // Update the round sync.
        round_sync.update_round_locators(ip, &round_locators).unwrap();

        // GC the round sync.
        let gc_round = round_locators.certificate_ids().keys().last().unwrap();
        round_sync.gc(*gc_round);
        assert!(round_sync.round_to_certificate_ids.read().is_empty());
        assert!(round_sync.certificate_id_to_round.read().is_empty());
        assert!(round_sync.certificate_id_to_ip.read().is_empty());
        assert_eq!(round_sync.gc_round.load(std::sync::atomic::Ordering::SeqCst), *gc_round);
    }
}
