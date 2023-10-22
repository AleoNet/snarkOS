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

use snarkvm::{
    console::prelude::*,
    ledger::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        narwhal::{Data, Transmission, TransmissionID},
    },
};

use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Ready<N: Network> {
    /// The current map of `(transmission ID, transmission)` entries.
    transmissions: Arc<RwLock<IndexMap<TransmissionID<N>, Transmission<N>>>>,
}

impl<N: Network> Default for Ready<N> {
    /// Initializes a new instance of the ready queue.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Ready<N> {
    /// Initializes a new instance of the ready queue.
    pub fn new() -> Self {
        Self { transmissions: Default::default() }
    }

    /// Returns `true` if the ready queue is empty.
    pub fn is_empty(&self) -> bool {
        self.transmissions.read().is_empty()
    }

    /// Returns the number of transmissions in the ready queue.
    pub fn num_transmissions(&self) -> usize {
        self.transmissions.read().len()
    }

    /// Returns the number of ratifications in the ready queue.
    pub fn num_ratifications(&self) -> usize {
        self.transmissions.read().keys().filter(|id| matches!(id, TransmissionID::Ratification)).count()
    }

    /// Returns the number of solutions in the ready queue.
    pub fn num_solutions(&self) -> usize {
        self.transmissions.read().keys().filter(|id| matches!(id, TransmissionID::Solution(..))).count()
    }

    /// Returns the number of transactions in the ready queue.
    pub fn num_transactions(&self) -> usize {
        self.transmissions.read().keys().filter(|id| matches!(id, TransmissionID::Transaction(..))).count()
    }

    /// Returns the transmission IDs in the ready queue.
    pub fn transmission_ids(&self) -> IndexSet<TransmissionID<N>> {
        self.transmissions.read().keys().copied().collect()
    }

    /// Returns the transmissions in the ready queue.
    pub fn transmissions(&self) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        self.transmissions.read().clone()
    }

    /// Returns the solutions in the ready queue.
    pub fn solutions(&self) -> impl '_ + Iterator<Item = (PuzzleCommitment<N>, Data<ProverSolution<N>>)> {
        self.transmissions.read().clone().into_iter().filter_map(|(id, transmission)| match (id, transmission) {
            (TransmissionID::Solution(id), Transmission::Solution(solution)) => Some((id, solution)),
            _ => None,
        })
    }

    /// Returns the transactions in the ready queue.
    pub fn transactions(&self) -> impl '_ + Iterator<Item = (N::TransactionID, Data<Transaction<N>>)> {
        self.transmissions.read().clone().into_iter().filter_map(|(id, transmission)| match (id, transmission) {
            (TransmissionID::Transaction(id), Transmission::Transaction(tx)) => Some((id, tx)),
            _ => None,
        })
    }
}

impl<N: Network> Ready<N> {
    /// Returns `true` if the ready queue contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmissions.read().contains_key(&transmission_id.into())
    }

    /// Returns the transmission, given the specified `transmission ID`.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Transmission<N>> {
        self.transmissions.read().get(&transmission_id.into()).cloned()
    }

    /// Inserts the specified (`transmission ID`, `transmission`) to the ready queue.
    /// Returns `true` if the transmission is new, and was added to the ready queue.
    pub fn insert(&self, transmission_id: impl Into<TransmissionID<N>>, transmission: Transmission<N>) -> bool {
        let transmission_id = transmission_id.into();
        // Insert the transmission ID.
        let is_new = self.transmissions.write().insert(transmission_id, transmission).is_none();
        // Return whether the transmission is new.
        is_new
    }

    /// Removes up to the specified number of transmissions and returns them.
    pub fn drain(&self, num_transmissions: usize) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        // Acquire the write lock.
        let mut transmissions = self.transmissions.write();
        // Determine the number of transmissions to drain.
        let range = 0..transmissions.len().min(num_transmissions);
        // Drain the transmission IDs.
        transmissions.drain(range).collect::<IndexMap<_, _>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::ledger::{coinbase::PuzzleCommitment, narwhal::Data};

    use ::bytes::Bytes;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    #[test]
    fn test_ready() {
        let rng = &mut TestRng::default();

        // Sample random fake bytes.
        let data = |rng: &mut TestRng| Data::Buffer(Bytes::from((0..512).map(|_| rng.gen::<u8>()).collect::<Vec<_>>()));

        // Initialize the ready queue.
        let ready = Ready::<CurrentNetwork>::new();

        // Initialize the commitments.
        let commitment_1 = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));
        let commitment_2 = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));
        let commitment_3 = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));

        // Initialize the solutions.
        let solution_1 = Transmission::Solution(data(rng));
        let solution_2 = Transmission::Solution(data(rng));
        let solution_3 = Transmission::Solution(data(rng));

        // Insert the commitments.
        assert!(ready.insert(commitment_1, solution_1.clone()));
        assert!(ready.insert(commitment_2, solution_2.clone()));
        assert!(ready.insert(commitment_3, solution_3.clone()));

        // Check the number of transmissions.
        assert_eq!(ready.num_transmissions(), 3);

        // Check the transmission IDs.
        let transmission_ids = vec![commitment_1, commitment_2, commitment_3].into_iter().collect::<IndexSet<_>>();
        assert_eq!(ready.transmission_ids(), transmission_ids);
        transmission_ids.iter().for_each(|id| assert!(ready.contains(*id)));

        // Check that an unknown commitment is not in the ready queue.
        let commitment_unknown = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));
        assert!(!ready.contains(commitment_unknown));

        // Check the transmissions.
        assert_eq!(ready.get(commitment_1), Some(solution_1.clone()));
        assert_eq!(ready.get(commitment_2), Some(solution_2.clone()));
        assert_eq!(ready.get(commitment_3), Some(solution_3.clone()));
        assert_eq!(ready.get(commitment_unknown), None);

        // Drain the ready queue.
        let transmissions = ready.drain(3);

        // Check the number of transmissions.
        assert!(ready.is_empty());
        // Check the transmission IDs.
        assert_eq!(ready.transmission_ids(), IndexSet::new());

        // Check the transmissions.
        assert_eq!(
            transmissions,
            vec![(commitment_1, solution_1), (commitment_2, solution_2), (commitment_3, solution_3)]
                .into_iter()
                .collect::<IndexMap<_, _>>()
        );
    }

    #[test]
    fn test_ready_duplicate() {
        use rand::RngCore;
        let rng = &mut TestRng::default();

        // Sample random fake bytes.
        let mut vec = vec![0u8; 512];
        rng.fill_bytes(&mut vec);
        let data = Data::Buffer(Bytes::from(vec));

        // Initialize the ready queue.
        let ready = Ready::<CurrentNetwork>::new();

        // Initialize the commitments.
        let commitment = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));

        // Initialize the solutions.
        let solution = Transmission::Solution(data);

        // Insert the commitments.
        assert!(ready.insert(commitment, solution.clone()));
        assert!(!ready.insert(commitment, solution));

        // Check the number of transmissions.
        assert_eq!(ready.num_transmissions(), 1);
    }
}
