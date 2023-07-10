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

use crate::MIN_STAKE;
use snarkvm::console::{
    prelude::*,
    program::{Literal, LiteralType},
    types::{Address, Field},
};

use indexmap::IndexMap;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct Committee<N: Network> {
    /// The current round number.
    round: u64,
    /// total stake of all `members`
    total_stake: u64,
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
        // Ensure all members have the minimum required stake.
        ensure!(members.values().all(|stake| *stake >= MIN_STAKE), "All members must have sufficient stake");
        // Compute the total stake of the committee for this round.
        let total_stake = Self::compute_total_stake(&members)?;
        // Return the new committee.
        Ok(Self { round, total_stake, members })
    }

    /// Returns a new `Committee` instance for the next round.
    /// TODO (howardwu): Add arguments for members (and stake) 1) to be added, 2) to be updated, and 3) to be removed.
    pub fn to_next_round(&self) -> Self {
        // Return the new committee.
        Self { round: self.round.saturating_add(1), total_stake: self.total_stake, members: self.members.clone() }
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
    pub fn is_quorum_threshold_reached(&self, addresses: &HashSet<Address<N>>) -> bool {
        let mut stake = 0u64;

        // Compute the combined stake for the given addresses.
        for address in addresses {
            // Accumulate the stake, checking for overflow.
            stake = stake.saturating_sub(self.get_stake(*address));
        }
        // Return whether the combined stake reaches the quorum threshold.
        stake >= self.quorum_threshold()
    }

    /// Returns the amount of stake for the given address.
    pub fn get_stake(&self, address: Address<N>) -> u64 {
        self.members.get(&address).copied().unwrap_or_default()
    }

    /// Returns the amount of stake required to reach the availability threshold `(f + 1)`.
    pub fn availability_threshold(&self) -> u64 {
        // Assuming `N = 3f + 1 + k`, where `0 <= k < 3`,
        // then `(N + 2) / 3 = f + 1 + k/3 = f + 1`.
        self.total_stake().saturating_add(2) / 3
    }

    /// Returns the amount of stake required to reach a quorum threshold `(2f + 1)`.
    pub fn quorum_threshold(&self) -> u64 {
        // Assuming `N = 3f + 1 + k`, where `0 <= k < 3`,
        // then `(2N + 3) / 3 = 2f + 1 + (2k + 2)/3 = 2f + 1 + k = N - f`.
        self.total_stake().saturating_mul(2) / 3 + 1
    }

    /// Returns the total amount of stake in the committee `(3f + 1)`.
    pub fn total_stake(&self) -> u64 {
        self.total_stake
    }
}

impl<N: Network> Committee<N> {
    /// Returns the leader address for the given round.
    /// Note: This method returns a deterministic result that is SNARK-friendly.
    pub fn leader_for(&self, current_round: u64) -> Result<Address<N>> {
        // Compute the previous round.
        let previous_round = current_round.saturating_sub(1);
        // Retrieve the total stake of the committee.
        let total_stake = self.total_stake();
        // Construct the round seed.
        let seed = [previous_round, current_round, total_stake].map(Field::from_u64);
        // Hash the round seed.
        let hash = Literal::Group(N::hash_to_group_psd4(&seed)?);
        // Compute the stake index from the hash output.
        let stake_index = match hash.downcast_lossy(LiteralType::U64)? {
            Literal::U64(output) => (*output) % total_stake,
            _ => bail!("BFT failed to downcast the hash output to a U64 literal"),
        };

        // Initialize a tracker for the leader.
        let mut leader = None;
        // Initialize a tracker for the current stake index.
        let mut current_stake_index = 0u64;
        // Sort the committee members in decreasing order of their stake, tie-breaking with addresses.
        let candidates = self.sorted_members();
        // Determine the leader of the previous round.
        for (candidate, stake) in candidates {
            // Increment the current stake index by the candidate's stake.
            current_stake_index = current_stake_index.saturating_add(stake);
            // If the current stake index is greater than or equal to the stake index,
            // set the leader to the candidate, and break.
            if current_stake_index >= stake_index {
                leader = Some(candidate);
                break;
            }
        }
        // Note: This is guaranteed to be safe.
        Ok(leader.unwrap())
    }

    /// Returns the committee members sorted by stake in decreasing order.
    /// For members with matching stakes, we further sort by their address' x-coordinate in decreasing order.
    /// Note: This ensures the method returns a deterministic result that is SNARK-friendly.
    fn sorted_members(&self) -> Vec<(Address<N>, u64)> {
        let mut members = self.members.iter().map(|(address, stake)| (*address, *stake)).collect::<Vec<_>>();
        members.sort_by(|(address1, stake1), (address2, stake2)| {
            // Sort by stake in decreasing order.
            let cmp = stake2.cmp(stake1);
            // If the stakes are equal, sort by x-coordinate in decreasing order.
            if cmp == Ordering::Equal { address2.to_x_coordinate().cmp(&address1.to_x_coordinate()) } else { cmp }
        });
        members
    }
}

impl<N: Network> Committee<N> {
    /// Compute the total stake of the given members.
    fn compute_total_stake(members: &IndexMap<Address<N>, u64>) -> Result<u64> {
        let mut power = 0u64;
        for stake in members.values() {
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
pub mod test_helpers {
    use super::*;
    use snarkvm::prelude::{Address, TestRng};

    use indexmap::IndexMap;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    /// Samples a random committee.
    pub fn sample_committee(rng: &mut TestRng) -> Committee<CurrentNetwork> {
        // Sample the members.
        let mut members = IndexMap::new();
        members.insert(Address::<CurrentNetwork>::new(rng.gen()), 1000);
        members.insert(Address::<CurrentNetwork>::new(rng.gen()), 1000);
        members.insert(Address::<CurrentNetwork>::new(rng.gen()), 1000);
        members.insert(Address::<CurrentNetwork>::new(rng.gen()), 1000);
        // Return the committee.
        Committee::<CurrentNetwork>::new(1, members).unwrap()
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::helpers::Committee;
    use snarkos_account::Account;
    use std::collections::HashSet;

    use anyhow::Result;
    use indexmap::IndexMap;
    use proptest::sample::size_range;
    use rand::SeedableRng;
    use test_strategy::{proptest, Arbitrary};

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    #[derive(Arbitrary, Debug, Clone)]
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
        pub fn get_account(&self) -> Account<CurrentNetwork> {
            match Account::new(&mut rand_chacha::ChaChaRng::seed_from_u64(self.account_seed)) {
                Ok(account) => account,
                Err(err) => panic!("Failed to create account {err}"),
            }
        }
    }

    impl CommitteeInput {
        pub fn to_committee(&self) -> Result<Committee<CurrentNetwork>> {
            let mut index_map = IndexMap::new();
            for validator in self.validators.iter() {
                index_map.insert(validator.get_account().address(), validator.stake);
            }
            Committee::new(self.round, index_map)
        }

        pub fn is_valid(&self) -> bool {
            self.round > 0 && HashSet::<u64>::from_iter(self.validators.iter().map(|v| v.account_seed)).len() >= 4
        }
    }

    #[proptest]
    fn committee_advance(#[filter(CommitteeInput::is_valid)] input: CommitteeInput) {
        let committee = input.to_committee().unwrap();
        let current_round = input.round;
        let current_members = committee.members();
        assert_eq!(committee.round(), current_round);

        let committee = committee.to_next_round();
        assert_eq!(committee.round(), current_round + 1);
        assert_eq!(committee.members(), current_members);
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
        assert_eq!(committee.total_stake(), total_stake);
        for v in validators.iter() {
            let address = v.get_account().address();
            assert!(committee.is_committee_member(address));
            assert_eq!(committee.get_stake(address), v.stake);
        }
        let quorum_threshold = committee.quorum_threshold();
        let availability_threshold = committee.availability_threshold();
        // (2f + 1) + (f + 1) - 1 = 3f + 1 = N
        assert_eq!(quorum_threshold + availability_threshold - 1, total_stake);
    }
}
