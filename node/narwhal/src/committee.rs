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
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

pub struct Committee<N: Network> {
    /// The current round number.
    round: AtomicU64,
    /// A map of `address` to `stake`.
    members: RwLock<HashMap<Address<N>, u64>>,
}

impl<N: Network> Committee<N> {
    /// Initializes a new `Committee` instance.
    pub fn new(round: u64) -> Self {
        Self { round: AtomicU64::new(round), members: Default::default() }
    }
}

impl<N: Network> Committee<N> {
    /// Returns the current round number.
    pub fn round(&self) -> u64 {
        self.round.load(Ordering::Relaxed)
    }

    /// Increments the round number.
    pub fn increment_round(&self) {
        self.round.fetch_add(1, Ordering::Relaxed);
    }
}

impl<N: Network> Committee<N> {
    /// Adds a member to the committee.
    pub fn add_member(&self, address: Address<N>, stake: u64) -> Result<()> {
        // Check if the member is already in the committee.
        if self.is_committee_member(address) {
            bail!("Validator {address} is already a committee member");
        }
        // Add the member to the committee.
        self.members.write().insert(address, stake);
        Ok(())
    }

    /// Returns the committee members alongside their stake.
    pub fn members(&self) -> &RwLock<HashMap<Address<N>, u64>> {
        &self.members
    }

    /// Returns the number of validators in the committee.
    pub fn committee_size(&self) -> usize {
        self.members.read().len()
    }

    /// Returns `true` if the given address is in the committee.
    pub fn is_committee_member(&self, address: Address<N>) -> bool {
        self.members.read().contains_key(&address)
    }

    /// Returns the amount of stake for the given address.
    pub fn get_stake(&self, address: Address<N>) -> u64 {
        self.members.read().get(&address).copied().unwrap_or_default()
    }

    /// Returns the amount of stake required to reach the availability threshold `(f + 1)`.
    pub fn availability_threshold(&self) -> Result<u64> {
        // Assuming `N = 3f + 1 + k`, where `0 <= k < 3`,
        // then `(N + 2) / 3 = f + 1 + k/3 = f + 1`.
        Ok(self.total_stake()?.saturating_add(2) / 3)
    }

    /// Returns the amount of stake required to reach a quorum threshold `(2f + 1)`.
    pub fn quorum_threshold(&self) -> Result<u64> {
        // Assuming `N = 3f + 1 + k`, where `0 <= k < 3`,
        // then `(2N + 3) / 3 = 2f + 1 + (2k + 2)/3 = 2f + 1 + k = N - f`.
        Ok(self.total_stake()?.saturating_mul(2) / 3 + 1)
    }

    /// Returns the total amount of stake in the committee `(3f + 1)`.
    pub fn total_stake(&self) -> Result<u64> {
        // Compute the total power of the committee.
        let mut power = 0u64;
        for stake in self.members.read().values() {
            // Accumulate the stake, checking for overflow.
            power = match power.checked_add(*stake) {
                Some(power) => power,
                None => bail!("Failed to calculate total stake - overflow detected"),
            };
        }
        Ok(power)
    }
}
