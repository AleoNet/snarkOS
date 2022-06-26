// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    validator::{Reward, Round, Score, Stake, Validator},
    Address,
};

use indexmap::IndexMap;

/// The validator set.
#[derive(Clone)]
pub struct Validators {
    validators: IndexMap<Address, Validator>,
}

impl Validators {
    /// Initializes a new validator set.
    pub fn new() -> Self {
        Self {
            validators: Default::default(),
        }
    }

    /// Returns the number of validators.
    pub fn num_validators(&self) -> usize {
        self.validators.len()
    }

    /// Returns the total amount staked.
    pub fn total_stake(&self) -> Stake {
        // Note: As the total supply cannot exceed 2^64, this is call to `sum` is safe.
        self.validators.values().map(Validator::stake).sum()
    }

    /// Returns the validator with the given address, if the validator exists.
    pub fn get(&self, address: &Address) -> Option<&Validator> {
        self.validators.get(address)
    }
}
