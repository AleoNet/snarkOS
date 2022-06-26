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

use anyhow::{bail, Result};
use indexmap::{map::Entry, IndexMap};

/// The type for representing rewards.
type Reward = u64;
/// The type for representing the round.
type Round = u64;
/// The type for representing the validator score.
type Score = i128;
/// The type for representing stake.
type Stake = u64;

/// A validator in the validator set.
#[derive(Clone)]
pub struct Validator {
    /// The address of the validator.
    id: Address,
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
    pub fn new(id: Address, stake: Stake) -> Self {
        Self {
            id,
            stake,
            score: stake as Score,
            staked: [(id, stake)].iter().copied().collect(),
            rewards: [(id, 0)].iter().copied().collect(),
            leader_in: Vec::new(),
            participated_in: Vec::new(),
            byzantine: 0,
        }
    }

    /// Returns the validator ID.
    pub const fn id(&self) -> &Address {
        &self.id
    }

    /// Returns the staked amount.
    pub const fn staked(&self) -> Stake {
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
    pub fn increment_stake_by(&mut self, staker: Address, amount: Stake) -> Result<()> {
        // Update the stake.
        match self.stake.checked_add(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected overflow incrementing stake"),
        }

        // Update the staked amount.
        let mut entry = self.staked.entry(staker).or_insert(0);
        match entry.checked_add(amount) {
            Some(staked) => *entry = staked,
            None => bail!("Detected overflow incrementing staked amount"),
        };

        Ok(())
    }

    /// Decrements the staked amount by the given amount.
    pub fn decrement_stake_by(&mut self, staker: Address, amount: Stake) -> Result<()> {
        // Update the stake.
        match self.stake.checked_sub(amount) {
            Some(staked) => self.stake = staked,
            None => bail!("Detected underflow decrementing stake"),
        }

        // Retrieve the staked amount.
        let mut entry = match self.staked.get_mut(&staker) {
            Some(entry) => entry,
            None => bail!("Detected staker is not staked with validator"),
        };

        // Update the staked amount.
        match entry.checked_sub(amount) {
            Some(staked) => *entry = staked,
            None => bail!("Detected underflow decrementing staked amount"),
        };

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
