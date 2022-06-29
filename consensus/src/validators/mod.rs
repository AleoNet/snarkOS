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

use anyhow::{anyhow, bail, ensure, Error, Result};
use core::ops::Deref;
use fixed::types::U64F64;
use indexmap::{map::Entry, IndexMap};

pub struct Bond {
    validator: Address,
    staker: Address,
    amount: Stake,
}

impl Bond {
    /// Returns the address of the validator.
    pub const fn validator(&self) -> &Address {
        &self.validator
    }
}

pub struct Unbond {
    validator: Address,
    staker: Address,
    amount: Stake,
}

pub struct UnbondValidator {
    validator: Address,
}

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
    /// The current round number of the network.
    round: Round,
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
            round: 0,
            total_supply: Supply::from_num(STARTING_SUPPLY),
            active_validators: Default::default(),
            inactive_validators: Default::default(),
            unbonding: Default::default(),
        }
    }

    /// Returns the current round number of the network.
    pub const fn round(&self) -> Round {
        self.round
    }

    /// Returns the total supply (in gates) of the network.
    pub const fn total_supply(&self) -> Supply {
        self.total_supply
    }

    /// Returns the total amount staked that is bonded.
    pub fn total_stake(&self) -> Stake {
        // Note: As the total supply cannot exceed 2^64, this call to `sum` is safe.
        self.active_validators.values().map(Validator::stake).sum()
    }

    /// Returns the total amount of stake that is unbonding.
    pub fn total_stake_unbonding(&self) -> u64 {
        // Note: As the total supply cannot exceed 2^64, this call to `sum` is safe.
        self.unbonding.keys().map(|&(_, _, stake)| stake).sum()
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
            // Log a helper message.
            if validator.stake().saturating_sub(amount) < MIN_VALIDATOR_BOND {
                eprintln!("Use the `unbond_validator` call instead to fully unbond a validator");
            }

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

impl Validators {
    /// Processes all bonding and unbonding requests sequentially, and returns the new leader of the round.
    fn round_start(&mut self, bonds: &[Bond], unbonds: &[Unbond], unbond_validators: &[UnbondValidator]) -> Result<Address> {
        // Clone the validator set.
        let mut validators = self.clone();

        // Initialize a vector for the addresses of the new validators.
        let mut new_validators = Vec::new();

        // Iterate through the bonds, to increase the total staked and determine which validators are new.
        for bond in bonds.iter() {
            // Determine if the validator is new.
            let is_new_validator = !validators.active_validators.contains_key(bond.validator());

            // Add the bonding stake to the validator set.
            validators.bond(bond.validator, bond.staker, bond.amount)?;

            // If the validator does not exist, store the validator address.
            if is_new_validator {
                new_validators.push(bond.validator());
            }
        }

        // If there are new validators, update their score.
        if !new_validators.is_empty() {
            // Compute the total stake.
            let total_stake = validators.total_stake().floor().to_num::<u64>();

            // Initialize each validator score as `-1.125 * total_stake`.
            let score = {
                // Cast the stake into an `i128` to compute the score.
                let stake = Score::from_num(total_stake);
                // Compute 1/8 of the total stake.
                let one_eighth = match stake.checked_shr(3) {
                    Some(one_eighth) => one_eighth,
                    None => bail!("Failed to compute 1/8 of the total stake"),
                };
                // Compute `stake + stake / 8`.
                let value = match stake.checked_add(one_eighth) {
                    Some(value) => value,
                    None => bail!("Failed to compute the score"),
                };
                // Negate the value to compute the score.
                match value.checked_neg() {
                    Some(score) => score,
                    None => bail!("Failed to compute the score"),
                }
            };
            // Ensure the score is 0 or negative.
            ensure!(score <= Score::ZERO, "Score must be 0 or negative");

            // Update the score for each newly bonding validator.
            for validator in new_validators {
                // Retrieve the validator from the validator set.
                let validator = validators
                    .active_validators
                    .get_mut(validator)
                    .ok_or_else(|| anyhow!("The validator does not exist"))?;

                // Update the score.
                validator.set_score(score);
            }
        }

        // Iterate through the unbonds, to decrement the unbonded stake.
        for unbond in unbonds.iter() {
            // Decrement the bonded stake for the validator.
            validators.unbond(unbond.validator, unbond.staker, unbond.amount)?;
        }

        // Iterate through the unbond validators, to unbond the validator.
        for unbond_validator in unbond_validators.iter() {
            // Unbond the validator.
            validators.unbond_validator(unbond_validator.validator)?;
        }

        // Determine the leader.
        let leader = validators.get_leader()?;

        // Update the validator set.
        *self = validators;

        Ok(leader)
    }

    /// Processes the end of the round, by updating the score of all validators.
    fn round_finish(&mut self, leader: Address) -> Result<()> {
        // TODO (howardwu): Update total supply and any unbonding. Also add processor for faults.

        // Clone the validator set.
        let mut validators = self.clone();

        // Retrieve the current round.
        let round = validators.round();
        // Retrieve the current leader.
        let current_leader = validators.get_leader()?;
        // Ensure the leader is the same as the current leader.
        ensure!(leader == current_leader, "The leader does not match the expected leader");

        // Compute the total stake.
        let total_stake = validators.total_stake().floor().to_num::<u64>();

        // Update each validator.
        validators.active_validators.values_mut().try_for_each(|validator| {
            // Increment the validator by their staked amount.
            validator.increment_score_by(Score::from_num(validator.stake().floor().to_num::<u64>()))?;
            // Store that the validator participated in the round.
            validator.set_participated_in(round);

            // If this validator was the leader, decrement by the total staked.
            if validator.address() == &leader {
                // Decrement the validator by the total stake.
                validator.decrement_score_by(Score::from_num(total_stake))?;
                // Store that the validator led this round.
                validator.set_leader_in(round);
            }

            Ok::<_, Error>(())
        })?;

        // Update the validator set.
        *self = validators;

        Ok(())
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

    #[test]
    fn test_unbond() {
        // Initialize the validator set.
        let mut validators = Validators::new();
        assert_eq!(validators.total_stake(), 0);
        assert_eq!(validators.num_validators(), 0);

        // Bond one validator.
        let address_0 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_0, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND));
        assert_eq!(validators.num_validators(), 1);

        // Ensure the minimum validator bond is maintained.
        assert!(validators.unbond(address_0, address_0, Stake::from_num(1)).is_err());
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND));

        // Add a delegator to the validator.
        let address_1 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_1, Stake::from_num(ONE_CREDIT)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND + ONE_CREDIT));
        assert_eq!(validators.num_validators(), 1);

        // Unbond the delegator.
        validators.unbond(address_0, address_1, Stake::from_num(ONE_CREDIT)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND));
        assert_eq!(validators.total_stake_unbonding(), ONE_CREDIT);
        assert_eq!(validators.num_validators(), 1);
    }

    #[test]
    fn test_unbond_validator() {
        // Initialize the validator set.
        let mut validators = Validators::new();
        assert_eq!(validators.total_stake(), 0);
        assert_eq!(validators.num_validators(), 0);

        // Bond one validator.
        let address_0 = Address::rand(&mut test_crypto_rng());
        validators.bond(address_0, address_0, Stake::from_num(MIN_VALIDATOR_BOND)).unwrap();
        assert_eq!(validators.total_stake(), Stake::from_num(MIN_VALIDATOR_BOND));
        assert_eq!(validators.num_validators(), 1);

        // Unbond the validator.
        assert!(validators.unbond_validator(address_0).is_ok());
        assert_eq!(validators.total_stake(), Stake::from_num(0));
        assert_eq!(validators.total_stake_unbonding(), MIN_VALIDATOR_BOND);
        assert_eq!(validators.num_validators(), 0);
    }
}
