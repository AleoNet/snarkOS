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

    let max = std::cmp::max(block_height_around_year_10.saturating_sub(block_height), 0);
    let anchor_reward = anchor_reward::<STARTING_SUPPLY, ANCHOR_TIME>();
    let factor = factor::<ANCHOR_TIMESTAMP, ANCHOR_TIME>(num_validators, timestamp, block_height);

    let reward = (max * anchor_reward) as f64 * 2f64.powf(-1f64 * factor);

    reward
}

/// Calculate the factor used in the target adjustment algorithm and coinbase reward.
fn factor<const ANCHOR_TIMESTAMP: u64, const ANCHOR_TIME: u64>(num_validators: u64, timestamp: u64, block_height: u64) -> f64 {
    let numerator: f64 = (timestamp as f64 - ANCHOR_TIMESTAMP as f64) - (block_height as f64 * ANCHOR_TIME as f64);
    let denominator = num_validators * ANCHOR_TIME;

    numerator as f64 / denominator as f64
}

/// Returns the estimated block height after a given number of years for a specific anchor time.
fn estimated_block_height(anchor_time: u64, num_years: u32) -> u64 {
    const SECONDS_IN_A_YEAR: u64 = 60 * 60 * 24 * 365;

    let estimated_blocks_in_a_year = SECONDS_IN_A_YEAR / anchor_time;

    estimated_blocks_in_a_year * num_years as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    const NUM_GATES_PER_CREDIT: u64 = 1_000_000; // 1 million gates == 1 credit
    const STARTING_SUPPLY: u64 = 1_000_000_000 * NUM_GATES_PER_CREDIT; // 1 quadrillion gates == 1 billion credits
    const STAKING_PERCENTAGE: f64 = 0.025f64; // 2.5%

    const ANCHOR_TIMESTAMP: u64 = 1640179531; // 2019-01-01 00:00:00 UTC
    const ANCHOR_TIME: u64 = 15; // 15 seconds

    #[test]
    fn test_anchor_reward() {
        let reward = anchor_reward::<STARTING_SUPPLY, ANCHOR_TIME>();
        assert_eq!(reward, 4);

        // Increasing the anchor time will increase the reward.
        let larger_reward = anchor_reward::<STARTING_SUPPLY, { ANCHOR_TIME + 1 }>();
        assert!(reward < larger_reward);

        // Decreasing the anchor time will decrease the reward.
        let smaller_reward = anchor_reward::<STARTING_SUPPLY, { ANCHOR_TIME - 1 }>();
        assert!(reward > smaller_reward);
    }

    #[test]
    fn test_staking_reward() {
        let reward = staking_reward::<STARTING_SUPPLY, ANCHOR_TIME>();
        assert_eq!(reward, 11_891_171);

        // Increasing the anchor time will increase the reward.
        let larger_reward = staking_reward::<STARTING_SUPPLY, { ANCHOR_TIME + 1 }>();
        assert!(reward < larger_reward);

        // Decreasing the anchor time will decrease the reward.
        let smaller_reward = staking_reward::<STARTING_SUPPLY, { ANCHOR_TIME - 1 }>();
        assert!(reward > smaller_reward);
    }

    #[test]
    fn test_coinbase_reward() {
        let estimated_blocks_in_10_years = estimated_block_height(ANCHOR_TIME, 10);

        let mut block_height = 1;
        let mut timestamp = ANCHOR_TIMESTAMP;

        let mut previous_reward =
            coinbase_reward::<STARTING_SUPPLY, ANCHOR_TIMESTAMP, ANCHOR_TIME>(NUM_GATES_PER_CREDIT, timestamp, block_height);

        block_height *= 2;
        timestamp = ANCHOR_TIMESTAMP + block_height * ANCHOR_TIME;

        while block_height < estimated_blocks_in_10_years {
            let reward = coinbase_reward::<STARTING_SUPPLY, ANCHOR_TIMESTAMP, ANCHOR_TIME>(NUM_GATES_PER_CREDIT, timestamp, block_height);
            assert!(reward <= previous_reward);

            previous_reward = reward;
            block_height *= 2;
            timestamp = ANCHOR_TIMESTAMP + block_height * ANCHOR_TIME;
        }
    }

    #[test]
    fn test_coinbase_reward_after_10_years() {
        let estimated_blocks_in_10_years = estimated_block_height(ANCHOR_TIME, 10);

        let mut block_height = estimated_blocks_in_10_years;

        for _ in 0..10 {
            let timestamp = ANCHOR_TIMESTAMP + block_height * ANCHOR_TIME;

            let reward = coinbase_reward::<STARTING_SUPPLY, ANCHOR_TIMESTAMP, ANCHOR_TIME>(NUM_GATES_PER_CREDIT, timestamp, block_height);

            assert_eq!(reward, 0.0);

            block_height *= 2;
        }
    }

    #[test]
    fn test_factor() {
        let num_validators = 100;
        let mut block_height = 1;

        for _ in 0..10 {
            // Factor is 0 when the timestamp is as expected for a given block height.
            let timestamp = ANCHOR_TIMESTAMP + block_height * ANCHOR_TIME;
            let f = factor::<ANCHOR_TIMESTAMP, ANCHOR_TIME>(num_validators, timestamp, block_height);
            assert_eq!(f, 0.0);

            // Factor greater than 0 when the timestamp is greater than expected for a given block height.
            let timestamp = ANCHOR_TIMESTAMP + (block_height + 1) * ANCHOR_TIME;
            let f = factor::<ANCHOR_TIMESTAMP, ANCHOR_TIME>(num_validators, timestamp, block_height);
            assert!(f > 0.0);

            // Factor less than 0 when the timestamp is less than expected for a given block height.
            let timestamp = ANCHOR_TIMESTAMP + (block_height - 1) * ANCHOR_TIME;
            let f = factor::<ANCHOR_TIMESTAMP, ANCHOR_TIME>(num_validators, timestamp, block_height);
            assert!(f < 0.0);

            block_height *= 2;
        }
    }
}
