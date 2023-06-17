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

use snarkvm::console::{prelude::*, types::Address};

use parking_lot::RwLock;
use std::collections::HashMap;

pub struct Shared<N: Network> {
    /// A map of `address` to `stake`.
    committee: RwLock<HashMap<Address<N>, u64>>,
}

impl<N: Network> Shared<N> {
    /// Initializes a new `Shared` instance.
    pub fn new() -> Self {
        Self { committee: RwLock::new(HashMap::new()) }
    }

    /// Adds a validator to the committee.
    pub fn add_validator(&self, address: Address<N>, stake: u64) -> Result<()> {
        // Check if the validator is already in the committee.
        if self.is_committee_member(&address) {
            bail!("Validator already in committee");
        }

        // Add the validator to the committee.
        self.committee.write().insert(address, stake);
        Ok(())
    }

    /// Returns the committee.
    pub fn committee(&self) -> &RwLock<HashMap<Address<N>, u64>> {
        &self.committee
    }

    /// Returns the number of validators in the committee.
    pub fn committee_size(&self) -> usize {
        self.committee.read().len()
    }

    /// Returns `true` if the given address is in the committee.
    pub fn is_committee_member(&self, address: &Address<N>) -> bool {
        self.committee.read().contains_key(address)
    }

    /// Returns the total amount of stake in the committee.
    pub fn total_stake(&self) -> Result<u64> {
        // Compute the total power of the committee.
        let mut power = 0u64;
        for stake in self.committee.read().values() {
            // Accumulate the stake, checking for overflow.
            power = match power.checked_add(*stake) {
                Some(power) => power,
                None => bail!("Failed to calculate total stake - overflow detected"),
            };
        }
        Ok(power)
    }

    /// Returns the amount of stake required to reach a quorum threshold `(2f + 1)`.
    pub fn quorum_threshold(&self) -> Result<u64> {
        // Assuming `N = 3f + 1 + k`, where `0 <= k < 3`,
        // then `(2N + 3) / 3 = 2f + 1 + (2k + 2)/3 = 2f + 1 + k = N - f`.
        Ok(self.total_stake()?.saturating_mul(2) / 3 + 1)
    }

    /// Returns the amount of stake required to reach the availability threshold `(f + 1)`.
    pub fn availability_threshold(&self) -> Result<u64> {
        // Assuming `N = 3f + 1 + k`, where `0 <= k < 3`,
        // then `(N + 2) / 3 = f + 1 + k/3 = f + 1`.
        Ok(self.total_stake()?.saturating_add(2) / 3)
    }
}
