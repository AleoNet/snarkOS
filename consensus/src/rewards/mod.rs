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

use snarkvm::prelude::Network;

// TODO (raychu86): Transition out of floating point arithmetic to something more precise.

/// Calculate the anchor reward.
pub fn anchor_reward<const STARTING_SUPPLY: u64, const ANCHOR_TIME: u64>() -> u64 {
    let block_height_around_year_10 = estimated_block_height(ANCHOR_TIME, 10);

    let numerator = 2 * STARTING_SUPPLY;
    let denominator = block_height_around_year_10 * (block_height_around_year_10 + 1);

    (numerator as f64 / denominator as f64).floor() as u64
}

/// Calculate the staking reward, given the starting supply and anchor time.
pub fn staking_reward<const STARTING_SUPPLY: u64, const ANCHOR_TIME: u64>() -> u64 {
    // The staking percentage at genesis.
    const STAKING_PERCENTAGE: f64 = 0.025f64; // 2.5%

    let block_height_around_year_1 = estimated_block_height(ANCHOR_TIME, 1);

    let reward = (STARTING_SUPPLY as f64 * STAKING_PERCENTAGE) / block_height_around_year_1 as f64;

    return reward.floor() as u64;
}

/// Calculate the coinbase reward for a given block.
pub fn coinbase_reward<const STARTING_SUPPLY: u64, const ANCHOR_TIMESTAMP: u64, const ANCHOR_TIME: u64>(
    num_validators: u64,
    timestamp: u64,
    block_height: u64,
) -> f64 {
    let block_height_around_year_10 = estimated_block_height(ANCHOR_TIME, 10);

    let max = std::cmp::max(block_height_around_year_10 - block_height, 0);
    let anchor_reward = anchor_reward::<STARTING_SUPPLY, ANCHOR_TIME>();
    let factor = factor::<ANCHOR_TIMESTAMP, ANCHOR_TIME>(num_validators, timestamp, block_height);

    let reward = (max * anchor_reward) as f64 * 2f64.powf(-1f64 * factor);

    reward
}

/// Calculate the factor used in the target adjustment algorithm and coinbase reward.
fn factor<const ANCHOR_TIMESTAMP: u64, const ANCHOR_TIME: u64>(num_validators: u64, timestamp: u64, block_height: u64) -> f64 {
    let numerator = (timestamp - ANCHOR_TIMESTAMP) - (block_height * ANCHOR_TIME);
    let denominator = num_validators * ANCHOR_TIME;

    numerator as f64 / denominator as f64
}

/// Returns the estimated block height after a given number of years for a specific anchor time.
fn estimated_block_height(anchor_time: u64, num_years: u32) -> u64 {
    const SECONDS_IN_A_YEAR: u64 = 60 * 60 * 24 * 365;

    let estimated_blocks_in_a_year = SECONDS_IN_A_YEAR / anchor_time;

    estimated_blocks_in_a_year * num_years as u64
}
