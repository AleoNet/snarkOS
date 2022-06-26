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
use indexmap::{map::Entry, IndexMap};

/// The type for representing rewards.
pub(super) type Reward = u64;
/// The type for representing the round.
pub(super) type Round = u64;
/// The type for representing the validator score.
pub(super) type Score = i128;
/// The type for representing stake.
pub(super) type Stake = u64;

/// A validator in the validator set.
#[derive(Clone)]
pub struct Validator {
    /// The address of the validator.
    address: Address,
    /// The amount of stake that the validator has bonded.
    stake: Stake,
    /// The score of the bonded validator.
    score: Score,
    /// The amount of stake that each staker (including the validator) has bonded.
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
            score: stake as Score,
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

    /// Returns the amount of stake that is bonded with this validator.
    pub const fn stake(&self) -> Stake {
        self.stake
    }

    /// Returns the validator score.
    pub const fn score(&self) -> Score {
        self.score
    }

    /// Returns the staked amount of the given staker.
    pub fn staked_by(&self, staker: &Address) -> Stake {
        self.staked.get(staker).copied().unwrap_or(0)
    }

    /// Returns the rewards amount of the given staker.
    pub fn rewards_by(&self, staker: &Address) -> Reward {
        self.rewards.get(staker).copied().unwrap_or(0)
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
    /// Increments the staked amount by the given amount.
    pub fn increment_stake_by(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker is bonding a nonzero amount.
        ensure!(amount > 0, "Staker must bond a nonzero amount");

        // Update the stake.
        match self.stake.checked_add(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected overflow incrementing stake"),
        }

        // Update the staked amount.
        let mut entry = self.staked.entry(*staker).or_insert(0);
        match entry.checked_add(amount) {
            Some(staked) => *entry = staked,
            None => bail!("Detected overflow incrementing staked amount"),
        };

        Ok(())
    }

    /// Decrements the staked amount by the given amount.
    pub fn decrement_stake_by(&mut self, staker: &Address, amount: Stake) -> Result<()> {
        // Ensure the staker exists.
        ensure!(self.staked.contains_key(staker), "Staker does not exist in validator");
        // Ensure the staker is unbonding a nonzero amount.
        ensure!(amount > 0, "Staker must unbond a nonzero amount");

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
    fn test_increment_stake_by() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let mut validator = Validator::new(address_0, 0);
        assert_eq!(validator.stake(), 0);
        assert_eq!(validator.staked_by(&address_0), 0);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure incrementing zero stake fails.
        assert!(validator.increment_stake_by(&address_0, 0).is_err());
        assert_eq!(validator.stake(), 0);
        assert_eq!(validator.staked_by(&address_0), 0);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure incrementing nonzero stake succeeds.
        assert!(validator.increment_stake_by(&address_0, 1).is_ok());
        assert_eq!(validator.stake(), 1);
        assert_eq!(validator.staked_by(&address_0), 1);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure incrementing updates the correct staker.
        assert!(validator.increment_stake_by(&address_1, 1).is_ok());
        assert_eq!(validator.stake(), 2);
        assert_eq!(validator.staked_by(&address_0), 1);
        assert_eq!(validator.staked_by(&address_1), 1);

        // Ensure incrementing to u64::MAX succeeds.
        assert!(validator.increment_stake_by(&address_0, u64::MAX - 2).is_ok());
        assert_eq!(validator.stake(), u64::MAX);
        assert_eq!(validator.staked_by(&address_0), u64::MAX - 1);
        assert_eq!(validator.staked_by(&address_1), 1);

        // Ensure incrementing past u64::MAX fails.
        assert!(validator.increment_stake_by(&address_0, 1).is_err());
        assert_eq!(validator.stake(), u64::MAX);
        assert_eq!(validator.staked_by(&address_0), u64::MAX - 1);
        assert_eq!(validator.staked_by(&address_1), 1);
    }

    #[test]
    fn test_decrement_stake_by() {
        let address_0 = Address::rand(&mut test_crypto_rng());
        let address_1 = Address::rand(&mut test_crypto_rng());

        let mut validator = Validator::new(address_0, 1_000_000);
        assert_eq!(validator.stake(), 1_000_000);
        assert_eq!(validator.staked_by(&address_0), 1_000_000);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure decrementing zero stake fails.
        assert!(validator.decrement_stake_by(&address_0, 0).is_err());
        assert_eq!(validator.stake(), 1_000_000);
        assert_eq!(validator.staked_by(&address_0), 1_000_000);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure decrementing nonzero stake succeeds.
        assert!(validator.decrement_stake_by(&address_0, 1).is_ok());
        assert_eq!(validator.stake(), 999_999);
        assert_eq!(validator.staked_by(&address_0), 999_999);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure decrementing nonexistent staker fails.
        assert!(validator.decrement_stake_by(&address_1, 1).is_err());
        assert_eq!(validator.stake(), 999_999);
        assert_eq!(validator.staked_by(&address_0), 999_999);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure decrementing below 0 fails.
        assert!(validator.decrement_stake_by(&address_0, 1_000_000).is_err());
        assert_eq!(validator.stake(), 999_999);
        assert_eq!(validator.staked_by(&address_0), 999_999);
        assert_eq!(validator.staked_by(&address_1), 0);

        // Ensure incrementing a new staker succeeds.
        assert!(validator.increment_stake_by(&address_1, 1).is_ok());
        assert_eq!(validator.stake(), 1_000_000);
        assert_eq!(validator.staked_by(&address_0), 999_999);
        assert_eq!(validator.staked_by(&address_1), 1);

        // Ensure decrementing updates the correct staker.
        assert!(validator.decrement_stake_by(&address_0, 1).is_ok());
        assert_eq!(validator.stake(), 999_999);
        assert_eq!(validator.staked_by(&address_0), 999_998);
        assert_eq!(validator.staked_by(&address_1), 1);

        // Ensure decrementing the validator to 0 succeeds.
        assert!(validator.decrement_stake_by(&address_0, 999_998).is_ok());
        assert_eq!(validator.stake(), 1);
        assert_eq!(validator.staked_by(&address_0), 0);
        assert_eq!(validator.staked_by(&address_1), 1);

        // Ensure underflow fails.
        assert!(validator.decrement_stake_by(&address_0, 1).is_err());
        assert_eq!(validator.stake(), 1);
        assert_eq!(validator.staked_by(&address_0), 0);
        assert_eq!(validator.staked_by(&address_1), 1);

        // Ensure decrementing the staker to 0 succeeds.
        assert!(validator.decrement_stake_by(&address_1, 1).is_ok());
        assert_eq!(validator.stake(), 0);
        assert_eq!(validator.staked_by(&address_0), 0);
        assert_eq!(validator.staked_by(&address_1), 0);
    }
}
