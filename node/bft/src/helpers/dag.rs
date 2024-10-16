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

use snarkvm::{
    console::types::{Address, Field},
    ledger::narwhal::BatchCertificate,
    prelude::Network,
};

use indexmap::IndexSet;
use std::collections::{BTreeMap, HashMap};

#[derive(Debug)]
pub struct DAG<N: Network> {
    /// The in-memory collection of certificates that comprise the DAG.
    graph: BTreeMap<u64, HashMap<Address<N>, BatchCertificate<N>>>,
    /// The in-memory collection of recently committed certificate IDs (up to GC).
    recent_committed_ids: BTreeMap<u64, IndexSet<Field<N>>>,
    /// The last round that was committed.
    last_committed_round: u64,
}

impl<N: Network> Default for DAG<N> {
    /// Initializes a new DAG.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> DAG<N> {
    /// Initializes a new DAG.
    pub fn new() -> Self {
        Self { graph: Default::default(), recent_committed_ids: Default::default(), last_committed_round: 0 }
    }

    /// Returns the DAG.
    pub const fn graph(&self) -> &BTreeMap<u64, HashMap<Address<N>, BatchCertificate<N>>> {
        &self.graph
    }

    /// Returns the last committed round.
    pub const fn last_committed_round(&self) -> u64 {
        self.last_committed_round
    }

    /// Returns `true` if the given certificate ID was recently committed.
    pub fn is_recently_committed(&self, round: u64, certificate_id: Field<N>) -> bool {
        self.recent_committed_ids.get(&round).map_or(false, |ids| ids.contains(&certificate_id))
    }

    /// Returns `true` if the given certificate ID exists in the given round.
    pub fn contains_certificate_in_round(&self, round: u64, certificate_id: Field<N>) -> bool {
        self.graph.get(&round).map_or(false, |map| map.values().any(|certificate| certificate.id() == certificate_id))
    }

    /// Returns the batch certificate for the given round and author.
    pub fn get_certificate_for_round_with_author(&self, round: u64, author: Address<N>) -> Option<BatchCertificate<N>> {
        self.graph.get(&round).and_then(|certificates| certificates.get(&author)).cloned()
    }

    /// Returns the batch certificate for the given round and certificate ID.
    pub fn get_certificate_for_round_with_id(
        &self,
        round: u64,
        certificate_id: Field<N>,
    ) -> Option<BatchCertificate<N>> {
        self.graph
            .get(&round)
            .and_then(|map| map.values().find(|certificate| certificate.id() == certificate_id))
            .cloned()
    }

    /// Returns the batch certificates for the given round.
    pub fn get_certificates_for_round(&self, round: u64) -> Option<HashMap<Address<N>, BatchCertificate<N>>> {
        self.graph.get(&round).cloned()
    }

    /// Inserts a certificate into the DAG.
    pub fn insert(&mut self, certificate: BatchCertificate<N>) {
        let round = certificate.round();
        let author = certificate.author();

        // If the certificate was not recently committed, insert it into the DAG.
        if !self.is_recently_committed(round, certificate.id()) {
            // Insert the certificate into the DAG.
            let _previous = self.graph.entry(round).or_default().insert(author, certificate);
            // If a previous certificate existed for the author, log it.
            #[cfg(debug_assertions)]
            if _previous.is_some() {
                error!("A certificate for round {round} by author {author} already existed in the DAG");
            }
        }
    }

    /// Commits a certificate, removing all certificates for this author at or before this round from the DAG.
    pub fn commit(&mut self, certificate: &BatchCertificate<N>, max_gc_rounds: u64) {
        let certificate_id = certificate.id();
        let certificate_round = certificate.round();
        let author = certificate.author();

        /* Updates */

        // Update the recently committed IDs.
        self.recent_committed_ids.entry(certificate_round).or_default().insert(certificate_id);

        // Update the last committed round.
        self.last_committed_round = self.last_committed_round.max(certificate_round);

        /* GC */

        // Remove committed IDs that are below the GC round.
        self.recent_committed_ids.retain(|round, _| round + max_gc_rounds > self.last_committed_round);

        // Remove certificates that are below the GC round.
        self.graph.retain(|round, _| round + max_gc_rounds > self.last_committed_round);
        // Remove any certificates for this author that are at or below the certificate round.
        self.graph.retain(|round, map| match *round > certificate_round {
            true => true,
            false => {
                map.remove(&author);
                !map.is_empty()
            }
        });
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;

    /// Returns a (malformed) mock DAG, with a modified `last_committed_round`.
    /// Note: This is useful **only** for testing purposes.
    pub(crate) fn mock_dag_with_modified_last_committed_round<N: Network>(last_committed_round: u64) -> DAG<N> {
        let mut dag = DAG::<N>::new();
        dag.last_committed_round = last_committed_round;
        dag
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::{
        prelude::{narwhal::batch_certificate::test_helpers::sample_batch_certificate_for_round, MainnetV0},
        utilities::TestRng,
    };

    #[test]
    fn test_dag_empty() {
        let dag = DAG::<MainnetV0>::new();

        assert_eq!(dag.get_certificates_for_round(0), None);
        assert_eq!(dag.last_committed_round(), 0);
    }

    #[test]
    fn test_dag_insert() {
        let rng = &mut TestRng::default();
        let mut dag = DAG::<MainnetV0>::new();

        const ROUND: u64 = 2;

        // Sample a certificate for round 2.
        let certificate = sample_batch_certificate_for_round(ROUND, rng);
        dag.insert(certificate.clone());
        assert!(dag.contains_certificate_in_round(ROUND, certificate.id()));
        assert_eq!(dag.get_certificate_for_round_with_author(ROUND, certificate.author()), Some(certificate.clone()));
        assert_eq!(dag.get_certificate_for_round_with_id(ROUND, certificate.id()), Some(certificate.clone()));
        assert_eq!(
            dag.get_certificates_for_round(ROUND),
            Some(vec![(certificate.author(), certificate)].into_iter().collect())
        );
        assert_eq!(dag.last_committed_round(), 0);
    }

    #[test]
    fn test_dag_commit() {
        let rng = &mut TestRng::default();
        let mut dag = DAG::<MainnetV0>::new();

        // Sample a certificate for round 2 and 3 with the same author.
        let certificate_2 = sample_batch_certificate_for_round(2, &mut TestRng::fixed(123456789));
        let certificate_3 = sample_batch_certificate_for_round(3, &mut TestRng::fixed(123456789));

        // Insert the certificate for round 2.
        dag.insert(certificate_2.clone());
        assert!(dag.contains_certificate_in_round(2, certificate_2.id()));
        assert_eq!(dag.get_certificate_for_round_with_author(2, certificate_2.author()), Some(certificate_2.clone()));
        assert_eq!(dag.get_certificate_for_round_with_id(2, certificate_2.id()), Some(certificate_2.clone()));
        assert_eq!(
            dag.get_certificates_for_round(2),
            Some(vec![(certificate_2.author(), certificate_2.clone())].into_iter().collect())
        );
        assert_eq!(dag.last_committed_round(), 0);

        // Insert the certificate for round 3.
        dag.insert(certificate_3.clone());
        assert!(dag.contains_certificate_in_round(3, certificate_3.id()));
        assert_eq!(dag.get_certificate_for_round_with_author(3, certificate_3.author()), Some(certificate_3.clone()));
        assert_eq!(dag.get_certificate_for_round_with_id(3, certificate_3.id()), Some(certificate_3.clone()));
        assert_eq!(
            dag.get_certificates_for_round(3),
            Some(vec![(certificate_3.author(), certificate_3.clone())].into_iter().collect())
        );
        assert_eq!(dag.last_committed_round(), 0);

        // Add a lower certificate. As the author is random, it's probably going to be different.
        let lower = sample_batch_certificate_for_round(2, rng);
        dag.insert(lower.clone());

        // Add a higher certificate. As the author is random, it's probably going to be different.
        let higher = sample_batch_certificate_for_round(4, rng);
        dag.insert(higher.clone());

        // Now commit the certificate for round 3, this will trigger GC.
        dag.commit(&certificate_3, 10);
        assert!(!dag.contains_certificate_in_round(2, certificate_2.id()));
        assert!(!dag.contains_certificate_in_round(3, certificate_3.id()));
        assert!(dag.contains_certificate_in_round(2, lower.id()));
        assert!(dag.contains_certificate_in_round(4, higher.id()));
        assert_eq!(dag.last_committed_round(), 3);
    }

    #[test]
    fn test_is_recently_committed() {
        let mut dag = DAG::<MainnetV0>::new();

        // Sample a certificate for round 2, 3, and 4 with the same author.
        let certificate_2 = sample_batch_certificate_for_round(2, &mut TestRng::fixed(123456789));
        let certificate_3 = sample_batch_certificate_for_round(3, &mut TestRng::fixed(123456789));
        let certificate_4 = sample_batch_certificate_for_round(4, &mut TestRng::fixed(123456789));

        /* Test normal case */

        // Insert the certificate for round 2.
        dag.insert(certificate_2.clone());
        assert!(!dag.is_recently_committed(2, certificate_2.id()));
        assert!(!dag.is_recently_committed(3, certificate_3.id()));

        // Now commit the certificate for round 2.
        dag.commit(&certificate_2, 2);
        assert!(dag.is_recently_committed(2, certificate_2.id()));
        assert!(!dag.is_recently_committed(3, certificate_3.id()));

        // Insert the certificate for round 3.
        dag.insert(certificate_3.clone());
        assert!(dag.is_recently_committed(2, certificate_2.id()));
        assert!(!dag.is_recently_committed(3, certificate_3.id()));

        // Now commit the certificate for round 3.
        dag.commit(&certificate_3, 2);
        assert!(dag.is_recently_committed(2, certificate_2.id()));
        assert!(dag.is_recently_committed(3, certificate_3.id()));

        /* Test GC case */

        // Insert the certificate for round 4.
        dag.insert(certificate_4.clone());
        assert!(dag.is_recently_committed(2, certificate_2.id()));
        assert!(dag.is_recently_committed(3, certificate_3.id()));
        assert!(!dag.is_recently_committed(4, certificate_4.id()));

        // Now commit the certificate for round 4.
        dag.commit(&certificate_4, 2);
        assert!(!dag.is_recently_committed(2, certificate_2.id()));
        assert!(dag.is_recently_committed(3, certificate_3.id()));
        assert!(dag.is_recently_committed(4, certificate_4.id()));
    }
}
