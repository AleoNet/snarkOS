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
    validator::{Round, Score, Stake, Validator},
    Address,
};

use anyhow::{anyhow, bail, ensure, Result};
use core::ops::Deref;
use fixed::types::U64F64;
use indexmap::{map::Entry, IndexMap};

/// The starting supply (in gates) of the network.
pub const STARTING_SUPPLY: u64 = 1_000_000_000_000_000;
/// The minimum stake (in gates) of a validator.
pub const MIN_STAKE: u64 = 1_000_000_000_000;

/// The unbonding period (in number of blocks) for a staker.
pub const UNBONDING_PERIOD: u32 = 40_320; // 4 * 60 * 24 * 7 ~= 1 week

/// The type for representing the supply (in gates).
pub(super) type Supply = U64F64;

/// The validator set.
#[derive(Clone)]
pub struct Validators {
    /// The total supply (in gates) of the network.
    total_supply: Supply,
    /// The active validators in the network.
    active_validators: IndexMap<Address, Validator>,
    /// The inactive validators in the network.
    inactive_validators: IndexMap<Address, Validator>,
    /// The map of unbonding stakes to the remaining number of blocks to wait,
    /// in the format of: `(validator, staker, stake) => remaining_blocks`
    unbonding: IndexMap<(Address, Address, u64), u32>,
}

impl Validators {
    /// Initializes a new validator set.
    pub fn new() -> Self {
        Self {
            total_supply: Supply::from_num(STARTING_SUPPLY),
            active_validators: Default::default(),
            inactive_validators: Default::default(),
            unbonding: Default::default(),
        }
    }

    /// Returns the total supply (in gates) of the network.
    pub const fn total_supply(&self) -> Supply {
        self.total_supply
    }

    /// Returns the total amount staked.
    pub fn total_stake(&self) -> Stake {
        // Note: As the total supply cannot exceed 2^64, this is call to `sum` is safe.
        self.active_validators.values().map(Validator::stake).sum()
    }

    /// Returns the number of validators.
    pub fn num_validators(&self) -> usize {
        self.active_validators.len()
    }

    /// Returns the validator with the given address, if the validator exists.
    pub fn get(&self, address: &Address) -> Option<&Validator> {
        self.active_validators.get(address)
    }

    /// Returns the current leader.
    pub fn get_leader(&self) -> Result<Address> {
        // Retrieve the validator with the highest score.
        let leader = self
            .active_validators
            .iter()
            .map(|(address, validator)| (address, validator.score()))
            .fold((None, Score::MIN), |(a, power_a), (b, power_b)| match power_a > power_b {
                true => (a, power_a),
                false => (Some(*b), power_b),
            })
            .0;

        // Return the leader address.
        match leader {
            Some(leader) => Ok(leader),
            None => bail!("No leader was found"),
        }
    }
}

impl Validators {
    /// Increments the bonded stake for the given validator address, as the given staker address with the given amount.
    pub(crate) fn bond(&mut self, validator: Address, staker: Address, amount: Stake) -> Result<()> {
        // If the validator does not exist, ensure the staker is the validator.
        if self.active_validators.get(&validator).is_none() {
            ensure!(
                validator == staker,
                "The staker must be the validator, as the validator does not exist yet"
            );
        }

        // Ensure the stake amount is nonzero.
        ensure!(amount > 0, "The stake amount must be nonzero");
        // Ensure the stake amount does not exceed 1/3 of the total supply.
        ensure!(
            amount <= self.total_supply / 3,
            "The stake must be less than 1/3 of the total supply"
        );

        // Retrieve the validator with the given address.
        match self.active_validators.entry(validator) {
            // If the validator exists, increment the bonded stake.
            Entry::Occupied(mut entry) => {
                // Retrieve the validator.
                let validator = entry.get_mut();

                // Ensure the validator does not exceed 1/3 of the total supply.
                ensure!(
                    validator.stake().saturating_add(amount) <= self.total_supply / 3,
                    "The validator stake must be less than 1/3 of the total supply"
                );

                // Increment the bonded stake for the validator.
                validator.increment_bonded_for(&staker, amount)
            }
            // If the validator does not exist, create a new validator.
            Entry::Vacant(entry) => {
                // Ensure the validator has the minimum required stake.
                ensure!(amount >= MIN_STAKE, "The validator must have the minimum required stake");

                // Create a new validator.
                entry.insert(Validator::new(validator, amount)?);
                Ok(())
            }
        }
    }

    /// Decrements the bonded stake for the given validator address, as the given staker address with the given amount.
    pub(crate) fn unbond(&mut self, validator: Address, staker: Address, amount: Stake) -> Result<()> {
        // Ensure the stake amount is nonzero.
        ensure!(amount > Stake::ZERO, "The stake amount must be nonzero");

        // Retrieve the validator from the validator set.
        let validator = self
            .active_validators
            .get_mut(&validator)
            .ok_or_else(|| anyhow!("The validator does not exist"))?;

        // If the staker is the validator, ensure they maintain the minimum required stake.
        if staker == *validator.address() {
            // Ensure the validator maintains the minimum required stake.
            ensure!(
                validator.stake().saturating_sub(amount) >= MIN_STAKE,
                "The validator must maintain the minimum required stake"
            );
        }

        // Decrement the bonded stake for the validator.
        validator.decrement_bonded_for(&staker, amount)?;

        // Add the staker and unbonded amount to the unbonding map.
        self.unbonding
            .insert((*validator.address(), staker, amount.floor().to_num()), UNBONDING_PERIOD);

        Ok(())
    }

    /// Decrements the bonded stake for the given validator address with the given amount.
    ///
    /// This method is used when a validator wishes to unbond their stake below the minimum required stake.
    /// This subsequently unbonds all stakers in the validator and removes the validator from the validator set.
    pub(crate) fn unbond_validator(&mut self, address: Address) -> Result<Validator> {
        // Retrieve the validator from the validator set.
        let validator = self
            .active_validators
            .get(&address)
            .ok_or_else(|| anyhow!("The validator does not exist"))?;

        // Retrieve the validator address.
        let validator_address = *validator.address();
        // Ensure the validator address is correct.
        ensure!(
            validator_address == address,
            "The validator address must be the same as the validator"
        );

        // Retrieve the stakers and their stake amounts from the validator.
        for (staker, stake) in validator.stakers()? {
            // Add the staker and unbonded amount to the unbonding map.
            self.unbonding
                .insert((validator_address, staker, stake.floor().to_num()), UNBONDING_PERIOD);
        }

        // Add the validator to the inactive validators.
        self.inactive_validators.insert(validator_address, validator.clone());

        // Remove the validator from the validator set.
        self.active_validators
            .remove(&validator_address)
            .ok_or_else(|| anyhow!("The validator could not be removed"))
    }
}

impl Deref for Validators {
    type Target = IndexMap<Address, Validator>;

    /// Returns the underlying validator map.
    fn deref(&self) -> &Self::Target {
        &self.active_validators
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_total_stake() {
        let validators = Validators::new();
        assert_eq!(validators.total_stake(), 0);
    }
}
