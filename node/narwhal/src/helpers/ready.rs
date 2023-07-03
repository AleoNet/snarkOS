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
    ledger::narwhal::{Transmission, TransmissionID},
};

use indexmap::IndexMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Ready<N: Network> {
    /// The map of `transmission IDs` to `transmissions`.
    transmissions: Arc<RwLock<IndexMap<TransmissionID<N>, Transmission<N>>>>,
    /// The cumulative proof target in the ready queue.
    cumulative_proof_target: Arc<RwLock<u128>>,
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
        Self { transmissions: Default::default(), cumulative_proof_target: Default::default() }
    }

    /// Returns the transmissions.
    pub const fn transmissions(&self) -> &Arc<RwLock<IndexMap<TransmissionID<N>, Transmission<N>>>> {
        &self.transmissions
    }

    /// Returns the cumulative proof target.
    pub fn cumulative_proof_target(&self) -> u128 {
        *self.cumulative_proof_target.read()
    }

    /// Returns the number of transmissions in the ready queue.
    pub fn len(&self) -> usize {
        self.transmissions.read().len()
    }

    /// Returns the transmission IDs.
    pub fn transmission_ids(&self) -> Vec<TransmissionID<N>> {
        self.transmissions.read().keys().copied().collect()
    }

    /// Returns `true` if the ready queue contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmissions.read().contains_key(&transmission_id.into())
    }

    /// Returns the transmission, given the specified `transmission ID`.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Transmission<N>> {
        self.transmissions.read().get(&transmission_id.into()).cloned()
    }

    /// Inserts the specified (`transmission ID`, `transmission`) to the ready queue.
    pub fn insert(&self, transmission_id: impl Into<TransmissionID<N>>, transmission: Transmission<N>) -> Result<()> {
        let transmission_id = transmission_id.into();
        // Check if the transmission ID is for a prover solution.
        if let TransmissionID::Solution(commitment) = &transmission_id {
            // Increment the cumulative proof target.
            let mut cumulative_proof_target = self.cumulative_proof_target.write();
            *cumulative_proof_target = cumulative_proof_target.saturating_add(commitment.to_target()? as u128);
            drop(cumulative_proof_target);
        }
        // Insert the transmission.
        self.transmissions.write().insert(transmission_id, transmission);
        Ok(())
    }

    /// Removes the specified `transmission ID` from the ready queue.
    pub fn remove(&self, transmission_id: impl Into<TransmissionID<N>>) -> Result<()> {
        let transmission_id = transmission_id.into();
        // Check if the transmission ID is for a prover solution.
        if let TransmissionID::Solution(commitment) = &transmission_id {
            // Decrement the cumulative proof target.
            let mut cumulative_proof_target = self.cumulative_proof_target.write();
            *cumulative_proof_target = cumulative_proof_target.saturating_sub(commitment.to_target()? as u128);
            drop(cumulative_proof_target);
        }
        // Remove the transmission.
        self.transmissions.write().remove(&transmission_id);
        Ok(())
    }

    /// Removes the transmissions and returns them.
    pub fn drain(&self) -> IndexMap<TransmissionID<N>, Transmission<N>> {
        // Acquire the write locks (simultaneously).
        let mut cumulative_proof_target = self.cumulative_proof_target.write();
        let mut transmissions = self.transmissions.write();
        // Reset the cumulative proof target.
        *cumulative_proof_target = 0;
        // Save the transmissions.
        let result = transmissions.clone();
        // Reset the transmissions.
        transmissions.clear();
        // Return the transmissions.
        result
    }
}
