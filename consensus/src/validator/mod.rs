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

use anyhow::{bail, ensure, Result};
use fixed::types::U64F64;
use indexmap::{map::Entry, IndexMap};

/// The type for representing rewards.
pub(super) type Reward = u64;
/// The type for representing the round.
pub(super) type Round = u64;
/// The type for representing the validator score.
pub(super) type Score = i128;
/// The type for representing stake (in gates).
pub(super) type Stake = U64F64;

/// A validator in the validator set.
#[derive(Clone)]
pub struct Validator {
    /// The address of the validator.
    address: Address,
    /// The amount of stake (in gates) held by this validator.
    stake: Stake,
    /// The score of the validator.
    score: Score,
    /// The amount of stake (in gates) that each staker (including the validator) has.
    staked: IndexMap<Address, Stake>,
    /// The amount of rewards that each staker (including the validator) has received.
    rewards: IndexMap<Address, Reward>,
    /// The round numbers that the validator successfully led.
    leader_in: Vec<Round>,
    /// The round numbers that the validator participated in (including rounds led by validator).
    participated_in: Vec<Round>,
    /// The probability (out of 100) that the validator behaves byzantine in a round.
    byzantine: u8,
}

impl Validator {
    /// Initializes a new validator with the given address and amount staked.
    pub fn new(address: Address, stake: Stake) -> Self {
        Self {
            address,
            stake,
            score: 0,
            staked: [(address, stake)].iter().copied().collect(),
            rewards: [(address, 0)].iter().copied().collect(),
            leader_in: Vec::new(),
            participated_in: Vec::new(),
            byzantine: 0,
        }
    }

    /// Returns the validator address.
    pub const fn address(&self) -> &Address {
        &self.address
    }

    /// Returns the amount of stake held by this validator.
    pub const fn stake(&self) -> Stake {
        self.stake
    }

    /// Returns the validator score.
    pub const fn score(&self) -> Score {
        self.score
    }

    /// Returns the staked amount of the given staker.
    pub fn staked_by(&self, staker: &Address) -> Stake {
        self.staked.get(staker).copied().unwrap_or_default()
    }

    /// Returns the rewards amount of the given staker.
    pub fn rewards_by(&self, staker: &Address) -> Reward {
        self.rewards.get(staker).copied().unwrap_or_default()
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
    /// Increments the staked amount by the given amount, incrementing each staker in this validator proportionally.
    pub(crate) fn increment_staked(&mut self, amount: Stake) -> Result<()> {
        // Ensure the staker is incrementing a nonzero amount.
        ensure!(amount > 0, "Staker must increment stake by a nonzero amount");

        // Ensure incrementing the stake does not overflow.
        ensure!(self.stake.checked_add(amount).is_some(), "Incrementing the stake overflows");

        // Initialize a vector to store the stakers and their proportional changes.
        let mut staked_increments = Vec::with_capacity(self.staked.len());

        // Ensure the increment for each staker succeeds.
        for (staker, stake) in &mut self.staked {
            // Compute the multiplier.
            let multiplier = *stake / self.stake;
            // Ensure the multiplier is nonzero.
            ensure!(multiplier > Stake::ZERO, "Multiplier is zero");

            // Compute the proportional change for the staker.
            let change = amount * multiplier;

            // Ensure incrementing does not overflow.
            ensure!(stake.checked_add(change).is_some(), "Incrementing the stake for staker overflows");

            // Add the staker and change to the vector.
            staked_increments.push((*staker, change));
        }

        // Ensure the sum of the staked increments equals the amount.
        let expected_amount = staked_increments.iter().map(|(_, amount)| amount).sum::<Stake>();
        ensure!(expected_amount == amount, "Sum of staked increments is incorrect");

        // Increment the stake for each staker.
        for (staker, change) in staked_increments {
            self.increment_staker_by(&staker, change)?;
        }

        Ok(())
    }

    /// Decrements the staked amount by the given amount, decrementing each staker in this validator proportionally.
    pub(crate) fn decrement_staked(&mut self, amount: Stake) -> Result<()> {
        // Ensure the staker is decrementing a nonzero amount.
        ensure!(amount > 0, "Staker must decrement stake by a nonzero amount");

        // Ensure decrementing the stake does not underflow.
        ensure!(self.stake.checked_sub(amount).is_some(), "Decrementing the stake underflows");

        // Initialize a vector to store the stakers and their proportional changes.
        let mut staked_decrements = Vec::with_capacity(self.staked.len());

        // Ensure the decrement for each staker succeeds.
        for (staker, stake) in &mut self.staked {
            // Compute the multiplier.
            let multiplier = *stake / self.stake;
            // Ensure the multiplier is nonzero.
            ensure!(multiplier > Stake::ZERO, "Multiplier is zero");

            // Compute the proportional change for the staker.
            let change = amount * multiplier;

            // Ensure decrementing does not underflow.
            ensure!(stake.checked_sub(change).is_some(), "Decrementing the stake for staker underflows");

            // Add the staker and change to the vector.
            staked_decrements.push((*staker, change));
        }

        // Ensure the sum of the staked decrements equals the amount.
        let expected_amount = staked_decrements.iter().map(|(_, amount)| amount).sum::<Stake>();
        ensure!(expected_amount == amount, "Sum of staked decrements is incorrect");

        // Decrement the stake for each staker.
        for (staker, change) in staked_decrements {
            self.decrement_staker_by(&staker, change)?;
        }

        Ok(())
    }
}

impl Validator {
    /// Increments the staked amount for the given staker by the given amount.
    /// This is to be used as an internal function only.
    fn increment_staker_by(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker is incrementing a nonzero amount.
        ensure!(amount > 0, "Staker must increment stake by a nonzero amount");

        // Update the stake.
        match self.stake.checked_add(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected overflow incrementing stake"),
        }

        // Update the staked amount.
        let mut entry = self.staked.entry(*staker).or_default();
        match entry.checked_add(amount) {
            Some(staked) => *entry = staked,
            None => bail!("Detected overflow incrementing staked amount"),
        };

        Ok(())
    }

    /// Decrements the staked amount for the given staker by the given amount.
    /// This is to be used as an internal function only.
    fn decrement_staker_by(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker exists.
        ensure!(self.staked.contains_key(staker), "Staker does not exist in validator");
        // Ensure the staker is decrementing a nonzero amount.
        ensure!(amount > 0, "Staker must decrement stake by a nonzero amount");

        // Retrieve the staked amount.
        let mut entry = match self.staked.get_mut(staker) {
            Some(entry) => entry,
            None => bail!("Detected staker is not staked with validator"),
        };

        // Update the staked amount.
        match entry.checked_sub(amount) {
            Some(staked) => *entry = staked,
            None => bail!("Detected underflow decrementing staked amount"),
        };

        // Update the stake.
        match self.stake.checked_sub(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected underflow decrementing stake"),
        }

        Ok(())
    }

    // /// Sets the score of the validator.
    // pub fn set_score(&mut self, score: Score) {
    //     self.score = score;
    // }

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

        let one_third = one / three;
        assert_eq!(one_third.to_string(), "0.3333333333333333333");

        // Ensure 0 != 1 / MAX.
        assert_ne!(Stake::ZERO, one / Stake::MAX);

        // These do **not** equal.
        let stake = Stake::MAX;
        assert_ne!(stake, Stake::from_num(u64::MAX));
    }

    #[test]
    fn test_increment_staker_by() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;

        // let u64_0 = 0;
        // let u64_1 = 1;

        let mut validator = Validator::new(address_0, u64_0);
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing zero stake fails.
        assert!(validator.increment_staker_by(&address_0, u64_0).is_err());
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing nonzero stake succeeds.
        assert!(validator.increment_staker_by(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing updates the correct staker.
        assert!(validator.increment_staker_by(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), 2);
        assert_eq!(validator.staked_by(&address_0), u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure incrementing to U64::MAX succeeds.
        assert!(validator.increment_staker_by(&address_0, Stake::MAX - u64_1 - u64_1).is_ok());
        assert_eq!(validator.stake(), Stake::MAX);
        assert_eq!(validator.staked_by(&address_0), Stake::MAX - u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure incrementing past U64::MAX fails.
        assert!(validator.increment_staker_by(&address_0, u64_1).is_err());
        assert_eq!(validator.stake(), Stake::MAX);
        assert_eq!(validator.staked_by(&address_0), Stake::MAX - u64_1);
        assert_eq!(validator.staked_by(&address_1), u64_1);
    }

    #[test]
    fn test_decrement_staker_by() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let u64_0 = Stake::ZERO;
        let u64_1 = Stake::ONE;
        let u64_1_000_000 = Stake::from_num(1_000_000);
        let u64_999_999 = Stake::from_num(999_999);
        let u64_999_998 = Stake::from_num(999_998);

        // let u64_0 = 0;
        // let u64_1 = 1;
        // let u64_999_998 = 999_998;
        // let u64_999_999 = 999_999;
        // let u64_1_000_000 = 1_000_000;

        let mut validator = Validator::new(address_0, u64_1_000_000);
        assert_eq!(validator.stake(), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_0), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure decrementing zero stake fails.
        assert!(validator.decrement_staker_by(&address_0, u64_0).is_err());
        assert_eq!(validator.stake(), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_0), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure decrementing nonzero stake succeeds.
        assert!(validator.decrement_staker_by(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure decrementing nonexistent staker fails.
        assert!(validator.decrement_staker_by(&address_1, u64_1).is_err());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure decrementing below 0 fails.
        assert!(validator.decrement_staker_by(&address_0, u64_1_000_000).is_err());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_0);

        // Ensure incrementing a new staker succeeds.
        assert!(validator.increment_staker_by(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_1_000_000);
        assert_eq!(validator.staked_by(&address_0), u64_999_999);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure decrementing updates the correct staker.
        assert!(validator.decrement_staker_by(&address_0, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_999_999);
        assert_eq!(validator.staked_by(&address_0), u64_999_998);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure decrementing the validator to 0 succeeds.
        assert!(validator.decrement_staker_by(&address_0, u64_999_998).is_ok());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure underflow fails.
        assert!(validator.decrement_staker_by(&address_0, u64_1).is_err());
        assert_eq!(validator.stake(), u64_1);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_1);

        // Ensure decrementing the staker to 0 succeeds.
        assert!(validator.decrement_staker_by(&address_1, u64_1).is_ok());
        assert_eq!(validator.stake(), u64_0);
        assert_eq!(validator.staked_by(&address_0), u64_0);
        assert_eq!(validator.staked_by(&address_1), u64_0);
    }
}
