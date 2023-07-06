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

use crate::helpers::Storage;
use snarkvm::{
    console::prelude::*,
    ledger::narwhal::{Transmission, TransmissionID},
};

use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Ready<N: Network> {
    /// The storage.
    storage: Storage<N>,
    /// The current set of `transmission IDs`.
    transmission_ids: Arc<RwLock<IndexSet<TransmissionID<N>>>>,
    /// The cumulative proof target in the ready queue.
    cumulative_proof_target: Arc<RwLock<u128>>,
}

impl<N: Network> Ready<N> {
    /// Initializes a new instance of the ready queue.
    pub fn new(storage: Storage<N>) -> Self {
        Self { storage, transmission_ids: Default::default(), cumulative_proof_target: Default::default() }
    }

    /// Returns `true` if the ready queue is empty.
    pub fn is_empty(&self) -> bool {
        self.transmission_ids.read().is_empty()
    }

    /// Returns the number of transmissions in the ready queue.
    pub fn len(&self) -> usize {
        self.transmission_ids.read().len()
    }

    /// Returns the cumulative proof target.
    pub fn cumulative_proof_target(&self) -> u128 {
        *self.cumulative_proof_target.read()
    }

    /// Returns the transmission IDs.
    pub fn transmission_ids(&self) -> IndexSet<TransmissionID<N>> {
        self.transmission_ids.read().clone()
    }

    /// Returns `true` if the ready queue contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmission_ids.read().contains(&transmission_id.into())
    }

    /// Returns the transmission, given the specified `transmission ID`.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Transmission<N>> {
        self.storage.get_transmission(transmission_id)
    }

    /// Inserts the specified (`transmission ID`, `transmission`) to the ready queue.
    /// Returns `true` if the transmission is new, and was added to the ready queue.
    pub fn insert(&self, transmission_id: impl Into<TransmissionID<N>>, transmission: Transmission<N>) -> Result<bool> {
        let transmission_id = transmission_id.into();

        // Determine if the transmission is new.
        let is_new = !self.contains(transmission_id) && !self.storage.contains_transmission(transmission_id);
        // If the transmission is new, insert it.
        if is_new {
            // Insert the transmission ID.
            self.transmission_ids.write().insert(transmission_id);
            // Insert the transmission.
            self.storage.insert_transmission(transmission_id, transmission);
            // Check if the transmission ID is for a prover solution.
            if let TransmissionID::Solution(commitment) = &transmission_id {
                // Increment the cumulative proof target.
                let mut cumulative_proof_target = self.cumulative_proof_target.write();
                *cumulative_proof_target = cumulative_proof_target.saturating_add(commitment.to_target()? as u128);
            }
        }
        // Return whether the transmission is new.
        Ok(is_new)
    }

    /// Removes the transmissions and returns them.
    pub fn drain(&self) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        // Scope the locks.
        let ids = {
            // Acquire the write locks (simultaneously).
            let mut cumulative_proof_target = self.cumulative_proof_target.write();
            let mut transmission_ids = self.transmission_ids.write();
            // Reset the cumulative proof target.
            *cumulative_proof_target = 0;
            // Drain the transmission IDs.
            transmission_ids.drain(..).collect::<Vec<_>>()
        };

        // Initialize a map for the transmissions.
        let mut transmissions = IndexMap::with_capacity(ids.len());
        // Iterate through the transmission IDs.
        for transmission_id in ids.iter() {
            // Retrieve the transmission.
            if let Some(transmission) = self.storage.get_transmission(*transmission_id) {
                // Insert the transmission.
                transmissions.insert(*transmission_id, transmission);
            }
        }
        // Return the transmissions.
        transmissions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::Storage;
    use snarkvm::ledger::{coinbase::PuzzleCommitment, narwhal::Data};

    use ::bytes::Bytes;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    #[test]
    fn test_ready() {
        let rng = &mut TestRng::default();

        // Sample random fake bytes.
        let data = |rng: &mut TestRng| Data::Buffer(Bytes::from((0..512).map(|_| rng.gen::<u8>()).collect::<Vec<_>>()));

        // Initialize the ready queue.
        let ready = Ready::<CurrentNetwork>::new(Storage::new(1));

        // Initialize the commitments.
        let commitment_1 = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));
        let commitment_2 = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));
        let commitment_3 = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));

        // Initialize the solutions.
        let solution_1 = Transmission::Solution(data(rng));
        let solution_2 = Transmission::Solution(data(rng));
        let solution_3 = Transmission::Solution(data(rng));

        // Insert the commitments.
        assert!(ready.insert(commitment_1, solution_1.clone()).unwrap());
        assert!(ready.insert(commitment_2, solution_2.clone()).unwrap());
        assert!(ready.insert(commitment_3, solution_3.clone()).unwrap());

        // Check the number of transmissions.
        assert_eq!(ready.len(), 3);

        // Compute the expected cumulative proof target.
        let expected_cumulative_proof_target = commitment_1.solution().unwrap().to_target().unwrap() as u128
            + commitment_2.solution().unwrap().to_target().unwrap() as u128
            + commitment_3.solution().unwrap().to_target().unwrap() as u128;

        // Check the cumulative proof target.
        assert_eq!(ready.cumulative_proof_target(), expected_cumulative_proof_target);

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
        let transmissions = ready.drain();

        // Check the number of transmissions.
        assert!(ready.is_empty());
        // Check the cumulative proof target.
        assert_eq!(ready.cumulative_proof_target(), 0);
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
        let ready = Ready::<CurrentNetwork>::new(Storage::new(1));

        // Initialize the commitments.
        let commitment = TransmissionID::Solution(PuzzleCommitment::from_g1_affine(rng.gen()));

        // Initialize the solutions.
        let solution = Transmission::Solution(data);

        // Insert the commitments.
        assert!(ready.insert(commitment, solution.clone()).unwrap());
        assert!(!ready.insert(commitment, solution).unwrap());

        // Check the number of transmissions.
        assert_eq!(ready.len(), 1);
    }
}
