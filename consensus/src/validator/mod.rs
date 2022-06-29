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

use crate::Address;

use anyhow::{anyhow, bail, ensure, Result};
use fixed::types::{I128F0, U64F64};
use indexmap::IndexMap;

/// The type for representing the round.
pub(super) type Round = u64;
/// The type for representing the validator score.
pub(super) type Score = I128F0;
/// The type for representing stake (in gates).
pub(super) type Stake = U64F64;

/// A conservative bound on the maximum amount of stake (in gates) that can be contributed at once.
/// The value is set to 2^52 to start as the total supply starts at 2^50 and a staker can
/// never contribute more than 1/3 of the total supply. Thus, this bound should never be hit.
const MAX_STAKE: u64 = 1 << 52;

/// A validator in the validator set.
#[derive(Clone)]
pub struct Validator {
    /// The address of the validator.
    address: Address,
    /// The amount of stake (in gates) held by this validator.
    stake: Stake,
    /// The score of the validator.
    score: Score,
    /// The amount of stake (in gates) that each staker (including the validator) has is comprised of the (bonded, earned) amounts.
    staked: IndexMap<Address, (Stake, Stake)>,
    /// The round numbers that the validator successfully led.
    leader_in: Vec<Round>,
    /// The round numbers that the validator participated in (including rounds led by validator).
    participated_in: Vec<Round>,
    /// The probability (out of 100) that the validator behaves byzantine in a round.
    byzantine: u8,
}

impl Validator {
    /// Initializes a new validator with the given address and amount staked.
    pub fn new(address: Address, stake: Stake) -> Result<Self> {
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(stake < MAX_STAKE, "Stake cannot exceed the maximum stake");

        // Compute the score as `-1.125 * stake`.
        let score = {
            // Cast the stake into an `i128` to compute the score.
            let stake = Score::from_num(stake.floor().to_num::<u64>());
            // Ensure the stake is less than the maximum stake as a sanity check.
            ensure!(stake < MAX_STAKE, "Stake cannot exceed the maximum stake");
            // Compute 1/8 of the stake.
            let one_eighth = match stake.checked_shr(3) {
                Some(one_eighth) => one_eighth,
                None => bail!("Failed to compute 1/8 of the given stake"),
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
        ensure!(score <= 0, "Score must be 0 or negative");

        Ok(Self {
            address,
            stake,
            score,
            staked: [(address, (stake, Stake::ZERO))].iter().copied().collect(),
            leader_in: Vec::new(),
            participated_in: Vec::new(),
            byzantine: 0,
        })
    }

    /// Returns the address of the validator.
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// Returns the amount of stake held by this validator, which is the sum of all bonded and earned stake.
    pub const fn stake(&self) -> Stake {
        self.stake
    }

    /// Returns the validator score, which is used for leader selection.
    pub const fn score(&self) -> Score {
        self.score
    }

    /// Returns the number of stakers (including the validator) for this validator.
    pub fn num_stakers(&self) -> usize {
        self.staked.len()
    }

    /// Returns `true`

    /// Returns the staked amount of the given staker, which is the sum of the bonded and earned stake.
    pub fn staked_by(&self, staker: &Address) -> Stake {
        self.bonded_by(staker) + self.earned_by(staker)
    }

    /// Returns the bonded amount of the given staker.
    pub fn bonded_by(&self, staker: &Address) -> Stake {
        self.staked.get(staker).copied().unwrap_or_default().0
    }

    /// Returns the earned amount of the given staker.
    pub fn earned_by(&self, staker: &Address) -> Stake {
        self.staked.get(staker).copied().unwrap_or_default().1
    }

    /// Returns the rounds in which the validator led.
    pub fn leader_in(&self) -> &[Round] {
        &self.leader_in
    }

    /// Returns the rounds in which the validator participated in (including rounds led by validator).
    pub fn participated_in(&self) -> &[Round] {
        &self.participated_in
    }
}

impl Validator {
    /// Increments the bonded amount for the given staker by the given amount.
    pub(crate) fn increment_bonded_for(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker is incrementing a nonzero amount.
        ensure!(amount > 0, "Staker must increment bonded stake by a nonzero amount");
        // Ensure the amount is less than the maximum stake as a sanity check.
        ensure!(amount < MAX_STAKE, "Amount cannot exceed the maximum stake");
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(self.stake < MAX_STAKE, "Stake cannot exceed the maximum stake");

        // Retrieve the bonded amount for the staker.
        let mut entry = self.staked.entry(*staker).or_default();
        // Ensure incrementing by the bonded amount does not overflow.
        ensure!(
            entry.0.checked_add(amount).is_some(),
            "Incrementing the bonded stake by the amount overflows"
        );

        // Update the stake.
        match self.stake.checked_add(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected overflow incrementing stake"),
        }

        // Update the bonded amount.
        match entry.0.checked_add(amount) {
            Some(bonded) => entry.0 = bonded,
            None => bail!("Detected overflow incrementing bonded amount"),
        };

        Ok(())
    }

    /// Decrements the bonded amount for the given staker by the given amount.
    pub(crate) fn decrement_bonded_for(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker exists.
        ensure!(self.staked.contains_key(staker), "Staker does not exist in validator");
        // Ensure the staker is decrementing a nonzero amount.
        ensure!(amount > 0, "Staker must decrement bonded stake by a nonzero amount");
        // Ensure the amount is less than the maximum stake as a sanity check.
        ensure!(amount < MAX_STAKE, "Amount cannot exceed the maximum stake");
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(self.stake < MAX_STAKE, "Stake cannot exceed the maximum stake");

        // Retrieve the staked amount.
        let mut entry = match self.staked.get_mut(staker) {
            Some(entry) => entry,
            None => bail!("Detected staker is not staked with validator"),
        };

        // Update the bonded amount.
        match entry.0.checked_sub(amount) {
            Some(staked) => entry.0 = staked,
            None => bail!("Detected underflow decrementing bonded amount"),
        };

        // Update the stake.
        match self.stake.checked_sub(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected underflow decrementing stake"),
        }

        Ok(())
    }

    /// Increments the earned staked by the given amount, incrementing the earned stake for each staker
    /// in the validator by their pro-rata defined as `(bonded_i + earned_i) / staked`.
    pub(crate) fn increment_earned_by(&mut self, amount: Stake) -> Result<()> {
        // Ensure the staker is incrementing a nonzero amount.
        ensure!(amount > 0, "Staker must increment stake by a nonzero amount");
        // Ensure the amount is less than the maximum stake as a sanity check.
        ensure!(amount < MAX_STAKE, "Amount cannot exceed the maximum stake");

        // Ensure the current stake is nonzero.
        ensure!(self.stake > 0, "Stake is zero");
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(self.stake < MAX_STAKE, "Stake cannot exceed the maximum stake");
        // Ensure incrementing the stake does not overflow.
        ensure!(self.stake.checked_add(amount).is_some(), "Incrementing the stake overflows");

        // Initialize a vector to store the stakers and their proportional changes.
        let mut staked_increments = Vec::with_capacity(self.staked.len());

        // Ensure the increment for each staker succeeds.
        for (staker, (bonded, earned)) in &mut self.staked {
            // Ensure the bonded stake of the staker does not exceed the sum of stakes.
            ensure!(*bonded <= self.stake, "Bonded stake of staker exceeds sum of stakes");
            // Ensure the earned stake of the staker does not exceed the sum of stakes.
            ensure!(*earned <= self.stake, "Earned stake of staker exceeds sum of stakes");

            // Compute the sum of the bonded and earned stake.
            let stake = bonded
                .checked_add(*earned)
                .ok_or_else(|| anyhow!("Sum of bonded & earned stake overflows"))?;
            // Ensure the stake of the staker does not exceed the sum of stakes.
            ensure!(stake <= self.stake, "Stake of staker exceeds sum of stakes");

            // Compute the stake multiplier.
            let stake_multiplier = stake / self.stake;
            // Ensure the stake multiplier is less than or equal to 1.
            ensure!(stake_multiplier <= Stake::ONE, "Stake multiplier is above 1");

            // Compute the stake change for the staker as `(amount * stake_i / stake_sum)`.
            let stake_change = amount * stake_multiplier;
            // Ensure incrementing by the change does not overflow.
            ensure!(
                stake.checked_add(stake_change).is_some(),
                "Incrementing the stake for staker overflows"
            );

            // Add the staker and stake change to the vector.
            staked_increments.push((*staker, stake_change));
        }

        // Compute the candidate amount, by summing the increments.
        let candidate_amount = staked_increments.iter().map(|(_, stake_change)| stake_change).sum::<Stake>();
        // Ensure the sum of the staked increments is less than or equal to the given amount.
        if candidate_amount > amount {
            bail!("Sum of increments must be <= {amount}: found {candidate_amount}")
        }
        // Ensure the candidate amount is off by one gate at most.
        ensure!(amount - candidate_amount < 1, "Sum of increments is off by more than one gate");

        // Increment the stake earned for each staker.
        for (staker, stake_change) in staked_increments {
            self.increment_earned_for(&staker, stake_change)?;
        }

        Ok(())
    }

    /// Decrements the staked amount by the given amount, decrementing the stake for each staker
    /// in the validator by their pro-rata defined as `(bonded_i + earned_i) / staked`.
    ///
    /// This method decrements the `earned` stake of each staker first, followed by the `bonded` stake.
    pub(crate) fn decrement_staked_by(&mut self, amount: Stake) -> Result<()> {
        // Ensure the staker is decrementing a nonzero amount.
        ensure!(amount > 0, "Staker must decrement stake by a nonzero amount");
        // Ensure the amount is less than the maximum stake as a sanity check.
        ensure!(amount < MAX_STAKE, "Amount cannot exceed the maximum stake");

        // Ensure the current stake is nonzero.
        ensure!(self.stake > 0, "Stake is zero");
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(self.stake < MAX_STAKE, "Stake cannot exceed the maximum stake");
        // Ensure decrementing the stake does not underflow.
        ensure!(self.stake.checked_sub(amount).is_some(), "Decrementing the stake underflows");

        // Initialize a vector to store the stakers and their proportional changes.
        let mut staked_decrements = Vec::with_capacity(self.staked.len());

        // Ensure the decrement for each staker succeeds.
        for (staker, (bonded, earned)) in &mut self.staked {
            // Ensure the bonded stake of the staker does not exceed the sum of stakes.
            ensure!(*bonded <= self.stake, "Bonded stake of staker exceeds sum of stakes");
            // Ensure the earned stake of the staker does not exceed the sum of stakes.
            ensure!(*earned <= self.stake, "Earned stake of staker exceeds sum of stakes");

            // Compute the sum of the bonded and earned stake.
            let stake = bonded
                .checked_add(*earned)
                .ok_or_else(|| anyhow!("Sum of bonded & earned stake overflows"))?;
            // Ensure the stake of the staker does not exceed the sum of stakes.
            ensure!(stake <= self.stake, "Stake of staker exceeds sum of stakes");

            // Compute the stake multiplier.
            let stake_multiplier = stake / self.stake;
            // Ensure the stake multiplier is less than or equal to 1.
            ensure!(stake_multiplier <= Stake::ONE, "Stake multiplier is above 1");

            // Compute the stake change for the staker as `(amount * stake_i / stake_sum)`.
            let stake_change = amount * stake_multiplier;
            // Ensure decrementing by the change does not underflow.
            ensure!(
                stake.checked_sub(stake_change).is_some(),
                "Decrementing the stake for staker underflows"
            );

            // Add the staker and stake change to the vector.
            match stake_change >= *earned {
                // Decrement from the earned first, then the remainder from the bonded.
                true => {
                    // Compute the bonded change.
                    let difference = stake_change.checked_sub(*earned);
                    // Ensure the bonded change does not underflow.
                    ensure!(difference.is_some(), "Bonded stake change does not underflow");
                    // Add the staker and stake change to the vector.
                    staked_decrements.push((*staker, difference, *earned))
                }
                // Decrement from the earned only.
                false => staked_decrements.push((*staker, None, stake_change)),
            }
        }

        // Compute the candidate amount, by summing the decrements.
        let candidate_amount = staked_decrements
            .iter()
            .map(|(_, bonded_change, earned_change)| match bonded_change {
                Some(bonded_change) => bonded_change.saturating_add(*earned_change),
                None => *earned_change,
            })
            .sum::<Stake>();
        // Ensure the sum of the staked decrements is less than or equal to the given amount.
        if candidate_amount > amount {
            bail!("Sum of decrements must be <= {amount}: found {candidate_amount}")
        }
        // Ensure the candidate amount is off by one gate at most.
        ensure!(amount - candidate_amount < 1, "Sum of decrements is off by more than one gate");

        // Decrement the stake for each staker.
        for (staker, bonded_change, earned_change) in staked_decrements {
            // Decrement the earned stake.
            self.decrement_earned_for(&staker, earned_change)?;
            // Decrement the bonded stake.
            if let Some(bonded_change) = bonded_change {
                self.decrement_bonded_for(&staker, bonded_change)?;
            }
        }

        Ok(())
    }
}

impl Validator {
    /// Increments the earned amount for the given staker by the given amount.
    /// This is to be used as an internal function only.
    fn increment_earned_for(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker is incrementing a nonzero amount.
        ensure!(amount > 0, "Staker must increment earned stake by a nonzero amount");
        // Ensure the amount is less than the maximum stake as a sanity check.
        ensure!(amount < MAX_STAKE, "Stake cannot exceed the maximum stake");
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(self.stake < MAX_STAKE, "Stake cannot exceed the maximum stake");

        // Update the stake.
        match self.stake.checked_add(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected overflow incrementing stake"),
        }

        // Update the earned amount.
        let mut entry = self.staked.entry(*staker).or_default();
        match entry.1.checked_add(amount) {
            Some(earned) => entry.1 = earned,
            None => bail!("Detected overflow incrementing earned amount"),
        };

        Ok(())
    }

    /// Decrements the earned amount for the given staker by the given amount.
    /// This is to be used as an internal function only.
    fn decrement_earned_for(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker exists.
        ensure!(self.staked.contains_key(staker), "Staker does not exist in validator");
        // Ensure the staker is decrementing a nonzero amount.
        ensure!(amount > 0, "Staker must decrement earned stake by a nonzero amount");
        // Ensure the amount is less than the maximum stake as a sanity check.
        ensure!(amount < MAX_STAKE, "Stake cannot exceed the maximum stake");
        // Ensure the stake is less than the maximum stake as a sanity check.
        ensure!(self.stake < MAX_STAKE, "Stake cannot exceed the maximum stake");

        // Retrieve the staked amount.
        let mut entry = match self.staked.get_mut(staker) {
            Some(entry) => entry,
            None => bail!("Detected staker is not staked with validator"),
        };

        // Update the earned amount.
        match entry.1.checked_sub(amount) {
            Some(staked) => entry.1 = staked,
            None => bail!("Detected underflow decrementing earned amount"),
        };

        // Update the stake.
        match self.stake.checked_sub(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected underflow decrementing stake"),
        }

        Ok(())
    }

    /// Increments the score by the given amount.
    pub fn increment_score_by(&mut self, amount: Score) -> Result<()> {
        match self.score.checked_add(amount) {
            Some(score) => self.score = score,
            None => bail!("Detected overflow incrementing score"),
        }
        Ok(())
    }

    /// Decrements the score by the given amount.
    pub fn decrement_score_by(&mut self, amount: Score) -> Result<()> {
        match self.score.checked_sub(amount) {
            Some(score) => self.score = score,
            None => bail!("Detected underflow decrementing score"),
        }
        Ok(())
    }

    /// Stores the given round number as one that the validator led.
    pub fn set_leader_in(&mut self, round: Round) {
        self.leader_in.push(round);
    }

    /// Stores the given round number as one that the validator participated in.
    pub fn set_participated_in(&mut self, round: Round) {
        self.participated_in.push(round);
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use snarkvm::console::prelude::*;

    #[test]
    fn test_stake_arithmetic() {
        let zero = Stake::default();
        assert_eq!(zero, Stake::ZERO);

        let one = Stake::from_num(1);
        assert_eq!(one, Stake::ONE);

        let two = Stake::from_num(2);
        assert_eq!(two, Stake::ONE + Stake::ONE);
        assert_eq!(two, one + one);

        let three = Stake::from_num(3);
        assert_eq!(three, two + one);
        assert_eq!(three, one + one + one);

        let one_half = one / two;
        assert_eq!(one_half, Stake::from_num(0.5));
        assert_eq!(one_half.to_string(), "0.5");
        assert_eq!(Stake::from_num(0.5).to_string(), "0.5");

        let one_third = one / three;
        assert_eq!(one_third.to_string(), "0.3333333333333333333");

        // Ensure 0 != 1 / MAX.
        assert_ne!(Stake::ZERO, one / Stake::MAX);

        // These do **not** equal.
        let stake = Stake::MAX;
        assert_ne!(stake, Stake::from_num(u64::MAX));
    }

    #[test]
    fn test_increment_bonded_for() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;

        let mut validator = Validator::new(address_0, u64_0).unwrap();
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure incrementing zero stake fails.
        assert!(validator.increment_bonded_for(&address_0, u64_0).is_err());
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure incrementing nonzero stake succeeds.
        assert!(validator.increment_bonded_for(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_1);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure incrementing updates the correct staker.
        assert!(validator.increment_bonded_for(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), 2);
        assert_eq!(validator.staked_by(&address_0), u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), u64_1);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure incrementing to MAX_STAKE succeeds.
        assert!(
            validator
                .increment_bonded_for(&address_0, Stake::from_num(MAX_STAKE) - u64_1 - u64_1)
                .is_ok()
        );
        assert_eq!(validator.stake(), Stake::from_num(MAX_STAKE));
        assert_eq!(validator.staked_by(&address_0), Stake::from_num(MAX_STAKE) - u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), Stake::from_num(MAX_STAKE) - u64_1);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure incrementing past MAX_STAKE fails.
        assert!(validator.increment_bonded_for(&address_0, u64_1).is_err());
        assert_eq!(validator.stake(), Stake::from_num(MAX_STAKE));
        assert_eq!(validator.staked_by(&address_0), Stake::from_num(MAX_STAKE) - u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), Stake::from_num(MAX_STAKE) - u64_1);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);
    }

    #[test]
    fn test_decrement_bonded_for() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;
        let u64_1_000_000 = Stake::from_num(1_000_000);
        let u64_999_999 = Stake::from_num(999_999);
        let u64_999_998 = Stake::from_num(999_998);

        let mut validator = Validator::new(address_0, u64_1_000_000).unwrap();
        assert_eq!(validator.stake(), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_0), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_1_000_000);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing zero stake fails.
        assert!(validator.decrement_earned_for(&address_0, u64_0).is_err());
        assert_eq!(validator.stake(), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_0), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_1_000_000);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing nonzero stake succeeds.
        assert!(validator.decrement_bonded_for(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_999_999);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing nonexistent staker fails.
        assert!(validator.decrement_bonded_for(&address_1, u64_1).is_err());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_999_999);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing below 0 fails.
        assert!(validator.decrement_bonded_for(&address_0, u64_1_000_000).is_err());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_999_999);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure incrementing a new staker succeeds.
        assert!(validator.increment_bonded_for(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), u64_999_999);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing updates the correct staker.
        assert!(validator.decrement_bonded_for(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_998);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), u64_999_998);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing the validator to 0 succeeds.
        assert!(validator.decrement_bonded_for(&address_0, u64_999_998).is_ok());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure underflow fails.
        assert!(validator.decrement_bonded_for(&address_0, u64_1).is_err());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_1);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_1);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);

        // Ensure decrementing the staker to 0 succeeds.
        assert!(validator.decrement_bonded_for(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.earned_by(&address_0), u64_0);
        assert_eq!(validator.earned_by(&address_1), u64_0);
    }

    #[test]
    fn test_increment_earned_by() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());
        let address_2 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;
        let u64_2 = Stake::from_num(2);
        let u64_3 = Stake::from_num(3);
        let u64_4 = Stake::from_num(4);
        let u64_5 = Stake::from_num(5);
        let u64_6 = Stake::from_num(6);
        let u64_10 = Stake::from_num(10);
        let u64_15 = Stake::from_num(15);
        let u64_16 = Stake::from_num(16);

        let mut validator = Validator::new(address_0, u64_0).unwrap();
        assert_eq!(validator.stake(), u64_0);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.num_stakers(), 1);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_2);
        assert_eq!(validator.num_stakers(), 2);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_2, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_3);
        assert_eq!(validator.num_stakers(), 3);

        // Increment the staked amount by 3 (increment each by 1).
        validator.increment_earned_by(u64_3).unwrap();
        assert!(validator.stake() <= u64_6);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_2), u64_0);
        assert!(validator.earned_by(&address_0) <= u64_2);
        assert!(validator.earned_by(&address_1) <= u64_2);
        assert!(validator.earned_by(&address_2) <= u64_2);

        // Decrement the first staker by 1.
        assert!(validator.decrement_earned_for(&address_0, u64_1).is_ok());
        assert!(validator.stake() <= u64_5);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_2), u64_0);
        assert!(validator.earned_by(&address_0) <= u64_1);
        assert!(validator.earned_by(&address_1) <= u64_2);
        assert!(validator.earned_by(&address_2) <= u64_2);

        // Increment the staked amount by 5 (increment first by 1, rest by 2).
        validator.increment_earned_by(u64_5).unwrap();
        assert!(validator.stake() <= u64_10);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_2), u64_0);
        assert!(validator.earned_by(&address_0) <= u64_2);
        assert!(validator.earned_by(&address_1) <= u64_4);
        assert!(validator.earned_by(&address_2) <= u64_4);

        // Increment the staked amount by 5 (increment first by 1, rest by 2).
        validator.increment_earned_by(u64_5).unwrap();
        assert!(validator.stake() <= u64_15);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_2), u64_0);
        assert!(validator.earned_by(&address_0) <= u64_3);
        assert!(validator.earned_by(&address_1) <= u64_6);
        assert!(validator.earned_by(&address_2) <= u64_6);

        // Increment the staked amount by 1 (increment first by 0.2, rest by 0.4).
        validator.increment_earned_by(u64_1).unwrap();
        assert!(validator.stake() <= u64_16);
        assert_eq!(validator.bonded_by(&address_0), u64_0);
        assert_eq!(validator.bonded_by(&address_1), u64_0);
        assert_eq!(validator.bonded_by(&address_2), u64_0);
        assert!(validator.earned_by(&address_0) <= Stake::from_num(3.2));
        assert!(validator.earned_by(&address_1) <= Stake::from_num(6.4));
        assert!(validator.earned_by(&address_2) <= Stake::from_num(6.4));
    }

    #[test]
    fn test_decrement_staked() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());
        let address_2 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;
        let u64_2 = Stake::from_num(2);
        let u64_3 = Stake::from_num(3);
        let u64_4 = Stake::from_num(4);
        let u64_5 = Stake::from_num(5);
        let u64_6 = Stake::from_num(6);
        let u64_10 = Stake::from_num(10);
        let u64_15 = Stake::from_num(15);
        let u64_16 = Stake::from_num(16);

        let mut validator = Validator::new(address_0, u64_0).unwrap();
        assert_eq!(validator.stake(), u64_0);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_2);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_2, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_3);

        // Decrement the staked amount by 3 (decrement each by 1).
        validator.decrement_staked_by(u64_3).unwrap();
        assert!(validator.stake() >= u64_0);
        assert!(validator.staked_by(&address_0) >= u64_0);
        assert!(validator.staked_by(&address_1) >= u64_0);
        assert!(validator.staked_by(&address_2) >= u64_0);
        assert_eq!(validator.num_stakers(), 3);

        // Set the staker to 1.
        assert!(validator.increment_earned_for(&address_0, u64_1).is_ok());
        assert!(validator.stake() >= u64_1);

        // Set the staker to 2.
        assert!(validator.increment_earned_for(&address_1, u64_2).is_ok());
        assert!(validator.stake() >= u64_3);

        // Set the staker to 3.
        assert!(validator.increment_earned_for(&address_2, u64_3).is_ok());
        assert!(validator.stake() >= u64_6);

        // Decrement the staked amount by 3 (decrement first by 0.5, decrement second by ).
        validator.decrement_staked_by(u64_3).unwrap();
        assert!(validator.stake() >= u64_3);
        assert!(validator.staked_by(&address_0) >= Stake::from_num(0.5));
        assert!(validator.staked_by(&address_1) >= u64_1);
        assert!(validator.staked_by(&address_2) >= Stake::from_num(1.5));
    }

    #[test]
    fn test_increment_earned_for() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;
        let u64_2 = Stake::from_num(2);

        let mut validator = Validator::new(address_0, u64_0).unwrap();
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing zero stake fails.
        assert!(validator.increment_earned_for(&address_0, u64_0).is_err());
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing nonzero stake succeeds.
        assert!(validator.increment_earned_for(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing updates the correct staker.
        assert!(validator.increment_earned_for(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_2);
        assert_eq!(validator.staked_by(&address_0), u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure incrementing to MAX_STAKE succeeds.
        assert!(
            validator
                .increment_earned_for(&address_0, Stake::from_num(MAX_STAKE) - u64_2)
                .is_ok()
        );
        assert_eq!(validator.stake(), Stake::from_num(MAX_STAKE));
        assert_eq!(validator.staked_by(&address_0), Stake::from_num(MAX_STAKE) - u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure incrementing past MAX_STAKE fails.
        assert!(validator.increment_earned_for(&address_0, u64_1).is_err());
        assert_eq!(validator.stake(), Stake::from_num(MAX_STAKE));
        assert_eq!(validator.staked_by(&address_0), Stake::from_num(MAX_STAKE) - u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);
    }
}
