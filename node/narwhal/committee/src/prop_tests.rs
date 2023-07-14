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

use crate::{Committee, MIN_STAKE};
use snarkos_account::Account;
use snarkvm::prelude::Result;

use indexmap::IndexMap;
use proptest::sample::size_range;
use rand::SeedableRng;
use std::{collections::HashSet, hash::Hash};
use test_strategy::{proptest, Arbitrary};

type CurrentNetwork = snarkvm::prelude::Testnet3;

#[derive(Arbitrary, Debug, Clone)]
pub struct CommitteeInput {
    #[strategy(0u64..)]
    pub round: u64,
    // Using a HashSet here guarantees we'll check the PartialEq implementation on the
    // `account_seed` and generate unique validators.
    #[any(size_range(0..32).lift())]
    pub validators: HashSet<Validator>,
}

#[derive(Arbitrary, Debug, Clone, Eq)]
pub struct Validator {
    #[strategy(..5_000_000_000u64)]
    pub stake: u64,
    account_seed: u64,
}

// Validators can have the same stake but shouldn't have the same account seed.
impl PartialEq for Validator {
    fn eq(&self, other: &Self) -> bool {
        self.account_seed == other.account_seed
    }
}

// Make sure the Hash matches PartialEq.
impl Hash for Validator {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.account_seed.hash(state);
    }
}

impl Validator {
    pub fn get_account(&self) -> Account<CurrentNetwork> {
        match Account::new(&mut rand_chacha::ChaChaRng::seed_from_u64(self.account_seed)) {
            Ok(account) => account,
            Err(err) => panic!("Failed to create account {err}"),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.stake >= MIN_STAKE
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
        self.round > 0
            && HashSet::<u64>::from_iter(self.validators.iter().map(|v| v.account_seed)).len() >= 4
            && self.validators.iter().all(|v| v.stake >= MIN_STAKE)
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

    assert_eq!(committee.num_members(), validators.len());
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
