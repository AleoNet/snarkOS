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
pub const STARTING_SUPPLY: u64 = 1_000_000_000_000_000; // 1B credits
/// The minimum bonded stake (in gates) of a validator.
pub const MIN_VALIDATOR_BOND: u64 = 1_000_000_000_000; // 1M credits
/// The minimum stake (in gates) of a delegator.
pub const ONE_CREDIT: u64 = 1_000_000; // 1 credit

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
    pub fn get_validator(&self, address: &Address) -> Option<&Validator> {
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
        if !self.active_validators.contains_key(&validator) {
            ensure!(
                validator == staker,
                "The staker must be the validator, as the validator does not exist yet"
            );
        }

        // Ensure the stake amount is at least one credit.
        ensure!(amount >= ONE_CREDIT, "The stake amount must be at least 1 credit");
        // Ensure the stake amount is less than 1/3 of the total supply.
        ensure!(
            amount < self.total_supply / 3,
            "The stake must be less than 1/3 of the total supply"
        );

        // Retrieve the validator with the given address.
        match self.active_validators.entry(validator) {
            // If the validator exists, increment the bonded stake.
            Entry::Occupied(mut entry) => {
                // Retrieve the validator.
                let validator = entry.get_mut();

                // Ensure the validator is less than 1/3 of the total supply.
                ensure!(
                    validator.stake().saturating_add(amount) < self.total_supply / 3,
                    "Stake must be less than 1/3 of the total supply"
                );

                // Increment the bonded stake for the validator.
                validator.increment_bonded_for(&staker, amount)
            }
            // If the validator does not exist, create a new validator.
            Entry::Vacant(entry) => {
                // Ensure the validator has the minimum required stake.
                ensure!(amount >= MIN_VALIDATOR_BOND, "The validator must have the minimum required stake");

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
                validator.stake().saturating_sub(amount) >= MIN_VALIDATOR_BOND,
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
        let stakers = validator.stakers()?;

        // Remove the validator from the validator set.
        let validator = self
            .active_validators
            .remove(&validator_address)
            .ok_or_else(|| anyhow!("The validator could not be removed"))?;

        // Add the validator to the inactive validators.
        self.inactive_validators.insert(validator_address, validator.clone());

        // Retrieve the unbonding amounts from the validator.
        for (staker, stake) in stakers {
            // Add the staker and unbonded amount to the unbonding map.
            self.unbonding
                .insert((validator_address, staker, stake.floor().to_num()), UNBONDING_PERIOD);
        }

        Ok(validator)
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
#[allow(deprecated)]
mod tests {
    use super::*;
    use snarkvm::console::prelude::*;

    #[test]
    fn test_get_total_stake() {
        // Initialize the validator set.
        let mut validators = Validators::new();
        assert_eq!(validators.total_stake(), 0);

        // Add one validator.
        let address_0 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_0, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND));

        // Add one delegator.
        let address_1 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_1, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(2 * MIN_VALIDATOR_BOND));

        // Add another validator.
        let address_2 = Address::rand(&mut test_crypto_rng());
        validators
            .bond(address_2, address_2, Stake::from_num(2 * MIN_VALIDATOR_BOND))
            .unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(4 * MIN_VALIDATOR_BOND));
    }

    #[test]
    fn test_get_total_supply() {
        // Initialize the validator set.
        let validators = Validators::new();
        assert_eq!(validators.total_supply(), STARTING_SUPPLY);
    }

    #[test]
    fn test_num_validators() {
        // Initialize the validator set.
        let mut validators = Validators::new();
        assert_eq!(validators.num_validators(), 0);

        // Add one validator.
        let address_0 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_0, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(validators.num_validators(), 1);

        // Add another validator.
        let address_1 = Address::rand(&mut test_crypto_rng());
        validators
            .bond(address_1, address_1, Stake::from_num(2 * MIN_VALIDATOR_BOND))
            .unwrap();
        assert_eq!(validators.num_validators(), 2);
    }

    #[test]
    fn test_bond() {
        // Initialize the validator set.
        let mut validators = Validators::new();
        assert_eq!(validators.total_stake(), 0);
        assert_eq!(validators.num_validators(), 0);

        // Bond one validator.
        let address_0 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_0, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND));
        assert_eq!(validators.num_validators(), 1);

        // Bond more to the same validator.
        validators.bond(address_0, address_0, Stake::from_num(ONE_CREDIT)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND + ONE_CREDIT));
        assert_eq!(validators.num_validators(), 1);

        // Ensure bonding 0 fails.
        assert!(validators.bond(address_0, address_0, Stake::from_num(0)).is_err());
        // Ensure bonding less than the minimum delegate amount fails.
        assert!(validators.bond(address_0, address_0, Stake::from_num(ONE_CREDIT - 1)).is_err());
        // Ensure bonding more than 1/3 of the total supply fails.
        assert!(
            validators
                .bond(
                    address_0,
                    address_0,
                    Stake::from_num((STARTING_SUPPLY / 3) - MIN_VALIDATOR_BOND - ONE_CREDIT + 1)
                )
                .is_err()
        );

        // Ensure bonding less than 1/3 of the total supply succeeds.
        assert!(
            validators
                .bond(
                    address_0,
                    address_0,
                    Stake::from_num((STARTING_SUPPLY / 3) - MIN_VALIDATOR_BOND - ONE_CREDIT)
                )
                .is_ok()
        );

        // Bond a new validator.
        let address_1 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_1, address_1, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(
            validators.total_stake(),
            Stake::from_num((STARTING_SUPPLY / 3) + MIN_VALIDATOR_BOND)
        );
        assert_eq!(validators.num_validators(), 2);

        // Add a delegator to the same validator.
        let address_2 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_1, address_2, Stake::from_num(ONE_CREDIT)).unwrap();
        assert_eq!(
            validators.total_stake(),
            Stake::from_num((STARTING_SUPPLY / 3) + MIN_VALIDATOR_BOND + ONE_CREDIT)
        );
        assert_eq!(validators.num_validators(), 2);
    }
}
