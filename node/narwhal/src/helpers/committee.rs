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

use indexmap::IndexMap;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct Committee<N: Network> {
    /// The current round number.
    round: u64,
    /// A map of `address` to `stake`.
    members: IndexMap<Address<N>, u64>,
}

impl<N: Network> Committee<N> {
    /// Initializes a new `Committee` instance.
    pub fn new(round: u64, members: IndexMap<Address<N>, u64>) -> Result<Self> {
        // Ensure the round is nonzero.
        ensure!(round > 0, "Round must be nonzero");
        // Ensure there are at least 4 members.
        ensure!(members.len() >= 4, "Committee must have at least 4 members");
        // Return the new committee.
        Ok(Self { round, members })
    }

    /// Returns a new `Committee` instance for the next round.
    /// TODO (howardwu): Add arguments for members (and stake) 1) to be added, 2) to be updated, and 3) to be removed.
    pub fn to_next_round(&self) -> Result<Self> {
        // Increment the round number.
        let Some(round) = self.round.checked_add(1) else {
            bail!("Overflow when incrementing round number in committee");
        };
        // Return the new committee.
        Ok(Self { round, members: self.members.clone() })
    }
}

impl<N: Network> Committee<N> {
    /// Returns the current round number.
    pub fn round(&self) -> u64 {
        self.round
    }

    /// Returns the committee members alongside their stake.
    pub fn members(&self) -> &IndexMap<Address<N>, u64> {
        &self.members
    }

    /// Returns the number of validators in the committee.
    pub fn committee_size(&self) -> usize {
        self.members.len()
    }

    /// Returns `true` if the given address is in the committee.
    pub fn is_committee_member(&self, address: Address<N>) -> bool {
        self.members.contains_key(&address)
    }

    /// Returns `true` if the combined stake for the given addresses reaches the quorum threshold.
    /// This method takes in a `HashSet` to guarantee that the given addresses are unique.
    pub fn is_quorum_threshold_reached(&self, addresses: &HashSet<Address<N>>) -> Result<bool> {
        // Compute the combined stake for the given addresses.
        let mut stake = 0u64;
        for address in addresses {
            stake = match stake.checked_add(self.get_stake(*address)) {
                Some(stake) => stake,
                None => bail!("Overflow when computing combined stake to check quorum threshold"),
            };
        }
        // Return whether the combined stake reaches the quorum threshold.
        Ok(stake >= self.quorum_threshold()?)
    }

    /// Returns the amount of stake for the given address.
    pub fn get_stake(&self, address: Address<N>) -> u64 {
        self.members.get(&address).copied().unwrap_or_default()
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
        for stake in self.members.values() {
            // Accumulate the stake, checking for overflow.
            power = match power.checked_add(*stake) {
                Some(power) => power,
                None => bail!("Failed to calculate total stake - overflow detected"),
            };
        }
        Ok(power)
    }
}

#[cfg(test)]
pub mod tests {
    use crate::helpers::Committee;
    use anyhow::Result;
    use indexmap::IndexMap;
    use proptest::sample::size_range;
    use rand::SeedableRng;
    use snarkos_account::Account;
    use snarkvm::prelude::Testnet3;
    use test_strategy::{proptest, Arbitrary};

    type N = Testnet3;

    #[derive(Arbitrary, Debug)]
    pub struct CommitteeInput {
        #[strategy(0u64..)]
        pub round: u64,
        #[any(size_range(0..32).lift())]
        pub validators: Vec<Validator>,
    }

    #[derive(Arbitrary, Debug, Clone)]
    pub struct Validator {
        #[strategy(..5_000_000_000u64)]
        pub stake: u64,
        account_seed: u64,
    }

    impl Validator {
        pub fn get_account(&self) -> Account<N> {
            match Account::new(&mut rand_chacha::ChaChaRng::seed_from_u64(self.account_seed)) {
                Ok(account) => account,
                Err(err) => panic!("Failed to create account {err}"),
            }
        }
    }

    impl CommitteeInput {
        pub fn to_committee(&self) -> Result<Committee<N>> {
            let mut index_map = IndexMap::new();
            for validator in self.validators.iter() {
                index_map.insert(validator.get_account().address(), validator.stake);
            }
            Committee::new(self.round, index_map)
        }

        pub fn is_valid(&self) -> bool {
            self.round > 0 && self.validators.len() >= 4
        }
    }

    #[proptest]
    fn committee_members(input: CommitteeInput) {
        let committee = match input.to_committee() {
            Ok(committee) => {
                assert!(input.is_valid());
                committee
            }
            Err(err) => {
                assert!(!input.is_valid());
                match err.to_string().as_str() {
                    "Round must be nonzero" => assert_eq!(input.round, 0),
                    "Committee must have at least 4 members" => assert!(input.validators.len() < 4),
                    _ => panic!("Unexpected error: {err}"),
                }
                return Ok(());
            }
        };

        let validators = input.validators;

        let mut total_stake = 0;
        for v in validators.iter() {
            total_stake += v.stake;
        }

        assert_eq!(committee.committee_size(), validators.len());
        assert_eq!(committee.total_stake().unwrap(), total_stake);
        for v in validators.iter() {
            let address = v.get_account().address();
            assert!(committee.is_committee_member(address));
            assert_eq!(committee.get_stake(address), v.stake);
        }
        let quorum_threshold = committee.quorum_threshold().unwrap();
        let availability_threshold = committee.availability_threshold().unwrap();
        // (2f + 1) + (f + 1) - 1 = 3f + 1 = N
        assert_eq!(quorum_threshold + availability_threshold - 1, total_stake);
    }
}
