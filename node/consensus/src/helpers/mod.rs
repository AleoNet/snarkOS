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

use super::*;

/// Calculate the staking reward, given the starting supply and anchor time.
///     R_staking = floor((0.025 * S) / H_Y1)
///     S = Starting supply.
///     H_Y1 = Anchor block height at year 1.
pub const fn staking_reward(starting_supply: u64, anchor_time: u16) -> u64 {
    // Compute the anchor block height at year 1.
    let anchor_height_at_year_1 = anchor_block_height(anchor_time, 1);
    // Compute the annual staking reward: (0.025 * S).
    let annual_staking_reward = (starting_supply / 1000) * 25;
    // Compute the staking reward: (0.025 * S) / H_Y1.
    annual_staking_reward / anchor_height_at_year_1 as u64
}

/// Calculates the coinbase reward for a given block.
///     R_coinbase = max(0, H_Y10 - H) * R_anchor * 2^(-1 * (D - B) / B).
///     R_anchor = Anchor reward.
///     H_Y10 = Anchor block height at year 10.
///     H = Current block height.
///     D = Time elapsed since the previous block.
///     B = Anchor block time.
pub fn coinbase_reward(
    previous_timestamp: i64,
    timestamp: i64,
    block_height: u32,
    starting_supply: u64,
    anchor_time: u16,
) -> Result<u64> {
    // Compute the anchor block height at year 10.
    let anchor_height_at_year_10 = anchor_block_height(anchor_time, 10);
    // Compute the anchor reward.
    let anchor_reward = anchor_reward(starting_supply, anchor_time);
    // Compute the remaining blocks until year 10, as a u64.
    let num_remaining_blocks_to_year_10 = anchor_height_at_year_10.saturating_sub(block_height) as u64;
    // Return the coinbase reward.
    match num_remaining_blocks_to_year_10.checked_mul(anchor_reward).ok_or_else(|| anyhow!("Anchor reward overflow"))? {
        // After the anchor block height at year 10, the coinbase reward is 0.
        0 => Ok(0),
        // Until the anchor block height at year 10, the coinbase reward is determined by this equation:
        //   (num_remaining_blocks_to_year_10 * anchor_reward) * 2^{-1 * ((timestamp - previous_timestamp) - ANCHOR_TIME) / ANCHOR_TIME}
        reward => retarget(reward, previous_timestamp, timestamp, anchor_time as u32, true, anchor_time),
    }
}

/// Calculates the anchor reward.
///     R_anchor = floor((2 * S) / (H_Y10 * (H_Y10 + 1))).
///     S = Starting supply.
///     H_Y10 = Expected block height at year 10.
const fn anchor_reward(starting_supply: u64, anchor_time: u16) -> u64 {
    // Calculate the anchor block height at year 10.
    let anchor_height_at_year_10 = anchor_block_height(anchor_time, 10) as u64;
    // Compute the numerator.
    let numerator = 2 * starting_supply;
    // Compute the denominator.
    let denominator = anchor_height_at_year_10 * (anchor_height_at_year_10 + 1);
    // Return the anchor reward.
    numerator / denominator
}

/// Returns the anchor block height after a given number of years for a specific anchor time.
pub const fn anchor_block_height(anchor_time: u16, num_years: u32) -> u32 {
    // Calculate the number of seconds in a year.
    const SECONDS_IN_A_YEAR: u32 = 60 * 60 * 24 * 365;
    // Calculate the one-year anchor block height.
    let anchor_block_height_at_year_1 = SECONDS_IN_A_YEAR / anchor_time as u32;
    // Return the anchor block height for the given number of years.
    anchor_block_height_at_year_1 * num_years
}

// TODO (raychu86): Remove `IS_V4` after Phase 2.
/// Calculate the coinbase target for the given block height.
pub fn coinbase_target<const IS_V4: bool>(
    previous_coinbase_target: u64,
    previous_block_timestamp: i64,
    block_timestamp: i64,
    anchor_time: u16,
    num_blocks_per_epoch: u32,
) -> Result<u64> {
    // Compute the half life.
    let half_life = if IS_V4 {
        num_blocks_per_epoch.saturating_div(2).saturating_mul(anchor_time as u32)
    } else {
        num_blocks_per_epoch
    };

    // Compute the new coinbase target.
    let candidate_target =
        retarget(previous_coinbase_target, previous_block_timestamp, block_timestamp, half_life, true, anchor_time)?;
    // Return the new coinbase target, floored at 2^10 - 1.
    Ok(core::cmp::max((1u64 << 10).saturating_sub(1), candidate_target))
}

/// Calculate the minimum proof target for the given coinbase target.
pub fn proof_target(coinbase_target: u64) -> u64 {
    coinbase_target.checked_shr(7).unwrap_or(7).saturating_add(1)
}

/// Retarget algorithm using fixed point arithmetic from https://www.reference.cash/protocol/forks/2020-11-15-asert.
///     T_{i+1} = T_i * 2^(INV * (D - B) / TAU).
///     T_i = Current target.
///     D = Time elapsed since the previous block.
///     B = Expected time per block.
///     TAU = Rate of doubling (or half-life) in seconds.
///     INV = {-1, 1} depending on whether the target is increasing or decreasing.
fn retarget(
    previous_target: u64,
    previous_block_timestamp: i64,
    block_timestamp: i64,
    half_life: u32,
    is_inverse: bool,
    anchor_time: u16,
) -> Result<u64> {
    // Compute the difference in block time elapsed, defined as:
    let mut drift = {
        // Determine the block time elapsed (in seconds) since the previous block.
        // Note: This operation includes a safety check for a repeat timestamp.
        let block_time_elapsed = core::cmp::max(block_timestamp.saturating_sub(previous_block_timestamp), 1);

        // Determine the difference in block time elapsed (in seconds).
        // Note: This operation must be *standard subtraction* to account for faster blocks.
        block_time_elapsed - anchor_time as i64
    };

    // If the drift is zero, return the previous target.
    if drift == 0 {
        return Ok(previous_target);
    }

    // Negate the drift if the inverse flag is set.
    if is_inverse {
        drift *= -1;
    }

    // Constants used for fixed point arithmetic.
    const RBITS: u32 = 16;
    const RADIX: u128 = 1 << RBITS;

    // Compute the exponent factor, and decompose it into integral & fractional parts for fixed point arithmetic.
    let (integral, fractional) = {
        // Calculate the exponent factor.
        let exponent = (RADIX as i128).saturating_mul(drift as i128) / half_life as i128;

        // Decompose into the integral and fractional parts.
        let integral = exponent >> RBITS;
        let fractional = (exponent - (integral << RBITS)) as u128;
        ensure!(fractional < RADIX, "Fractional part is not within the fixed point size");
        ensure!(exponent == (integral * (RADIX as i128) + fractional as i128), "Exponent is decomposed incorrectly");

        (integral, fractional)
    };

    // Approximate the fractional multiplier as 2^RBITS * 2^fractional, where:
    // 2^x ~= (1 + 0.695502049*x + 0.2262698*x**2 + 0.0782318*x**3)
    let fractional_multiplier = RADIX
        + ((195_766_423_245_049_u128 * fractional
            + 971_821_376_u128 * fractional.pow(2)
            + 5_127_u128 * fractional.pow(3)
            + 2_u128.pow(RBITS * 3 - 1))
            >> (RBITS * 3));

    // Cast the previous coinbase target from a u64 to a u128.
    // The difficulty target must allow for leading zeros to account for overflows;
    // an additional 64-bits for the leading zeros suffices.
    let candidate_target = (previous_target as u128).saturating_mul(fractional_multiplier);

    // Calculate the new difficulty.
    // Shift the target to multiply by 2^(integer) / RADIX.
    let shifts = integral - RBITS as i128;
    let mut candidate_target = if shifts < 0 {
        match candidate_target.checked_shr((-shifts) as u32) {
            Some(target) => core::cmp::max(target, 1),
            None => 1,
        }
    } else {
        match candidate_target.checked_shl(shifts as u32) {
            Some(target) => core::cmp::max(target, 1),
            None => u64::MAX as u128,
        }
    };

    // Cap the target at `u64::MAX` if it has overflowed.
    candidate_target = core::cmp::min(candidate_target, u64::MAX as u128);

    // Ensure that the leading 64 bits are zeros.
    ensure!(candidate_target.checked_shr(64) == Some(0), "The target has overflowed");
    // Cast the new target down from a u128 to a u64.
    Ok(candidate_target as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::{console::network::Testnet3, prelude::TestRng};

    use rand::Rng;

    type CurrentNetwork = Testnet3;

    const ITERATIONS: usize = 1000;

    const EXPECTED_ANCHOR_REWARD: u64 = 13;
    const EXPECTED_STAKING_REWARD: u64 = 21_800_481;
    const EXPECTED_COINBASE_REWARD_FOR_BLOCK_1: u64 = 163_987_187;

    #[test]
    fn test_anchor_reward() {
        let reward = anchor_reward(CurrentNetwork::STARTING_SUPPLY, CurrentNetwork::ANCHOR_TIME);
        assert_eq!(reward, EXPECTED_ANCHOR_REWARD);

        // Increasing the anchor time will increase the reward.
        let larger_reward = anchor_reward(CurrentNetwork::STARTING_SUPPLY, CurrentNetwork::ANCHOR_TIME + 1);
        assert!(reward < larger_reward);

        // Decreasing the anchor time will decrease the reward.
        let smaller_reward = anchor_reward(CurrentNetwork::STARTING_SUPPLY, CurrentNetwork::ANCHOR_TIME - 1);
        assert!(reward > smaller_reward);
    }

    #[test]
    fn test_staking_reward() {
        let reward = staking_reward(CurrentNetwork::STARTING_SUPPLY, CurrentNetwork::ANCHOR_TIME);
        assert_eq!(reward, EXPECTED_STAKING_REWARD);

        // Increasing the anchor time will increase the reward.
        let larger_reward = staking_reward(CurrentNetwork::STARTING_SUPPLY, CurrentNetwork::ANCHOR_TIME + 1);
        assert!(reward < larger_reward);

        // Decreasing the anchor time will decrease the reward.
        let smaller_reward = staking_reward(CurrentNetwork::STARTING_SUPPLY, CurrentNetwork::ANCHOR_TIME - 1);
        assert!(reward > smaller_reward);
    }

    #[test]
    fn test_coinbase_reward() {
        let reward = coinbase_reward(
            CurrentNetwork::GENESIS_TIMESTAMP,
            CurrentNetwork::GENESIS_TIMESTAMP + CurrentNetwork::ANCHOR_TIME as i64,
            1,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();
        assert_eq!(reward, EXPECTED_COINBASE_REWARD_FOR_BLOCK_1);

        // Increasing the block time to twice the anchor time *at most* halves the reward.
        let smaller_reward = coinbase_reward(
            CurrentNetwork::GENESIS_TIMESTAMP,
            CurrentNetwork::GENESIS_TIMESTAMP + (2 * CurrentNetwork::ANCHOR_TIME as i64),
            1,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();
        assert!(smaller_reward >= reward / 2);

        // Increasing the block time beyond the anchor time will decrease the reward.
        let smaller_reward = coinbase_reward(
            CurrentNetwork::GENESIS_TIMESTAMP,
            CurrentNetwork::GENESIS_TIMESTAMP + CurrentNetwork::ANCHOR_TIME as i64 + 1,
            1,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();
        assert!(reward > smaller_reward);

        // Decreasing the block time below the anchor time will increase the reward.
        let larger_reward = coinbase_reward(
            CurrentNetwork::GENESIS_TIMESTAMP,
            CurrentNetwork::GENESIS_TIMESTAMP + CurrentNetwork::ANCHOR_TIME as i64 - 1,
            1,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();
        assert!(reward < larger_reward);

        // Decreasing the block time to 0 *at most* doubles the reward.
        let larger_reward = coinbase_reward(
            CurrentNetwork::GENESIS_TIMESTAMP,
            CurrentNetwork::GENESIS_TIMESTAMP,
            1,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();
        assert!(larger_reward <= reward * 2);
    }

    #[test]
    fn test_coinbase_reward_up_to_year_10() {
        let anchor_height_at_year_10 = anchor_block_height(CurrentNetwork::ANCHOR_TIME, 10);

        let mut block_height = 1;
        let mut previous_timestamp = CurrentNetwork::GENESIS_TIMESTAMP;
        let mut timestamp = CurrentNetwork::GENESIS_TIMESTAMP;

        let mut previous_reward = coinbase_reward(
            previous_timestamp,
            timestamp,
            block_height,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();

        block_height *= 2;
        timestamp = CurrentNetwork::GENESIS_TIMESTAMP + block_height as i64 * CurrentNetwork::ANCHOR_TIME as i64;

        while block_height < anchor_height_at_year_10 {
            let reward = coinbase_reward(
                previous_timestamp,
                timestamp,
                block_height,
                CurrentNetwork::STARTING_SUPPLY,
                CurrentNetwork::ANCHOR_TIME,
            )
            .unwrap();
            assert!(reward <= previous_reward);

            previous_reward = reward;
            previous_timestamp = timestamp;
            block_height *= 2;
            timestamp = CurrentNetwork::GENESIS_TIMESTAMP + block_height as i64 * CurrentNetwork::ANCHOR_TIME as i64;
        }
    }

    #[test]
    fn test_coinbase_reward_after_year_10() {
        let mut rng = TestRng::default();

        let anchor_height_at_year_10 = anchor_block_height(CurrentNetwork::ANCHOR_TIME, 10);

        // Check that block `anchor_height_at_year_10` has a reward of 0.
        let reward = coinbase_reward(
            CurrentNetwork::GENESIS_TIMESTAMP,
            CurrentNetwork::GENESIS_TIMESTAMP + CurrentNetwork::ANCHOR_TIME as i64,
            anchor_height_at_year_10,
            CurrentNetwork::STARTING_SUPPLY,
            CurrentNetwork::ANCHOR_TIME,
        )
        .unwrap();
        assert_eq!(reward, 0);

        // Check that the subsequent blocks have a reward of 0.
        for _ in 0..ITERATIONS {
            let block_height: u32 = rng.gen_range(anchor_height_at_year_10..anchor_height_at_year_10 * 10);

            let timestamp =
                CurrentNetwork::GENESIS_TIMESTAMP + block_height as i64 * CurrentNetwork::ANCHOR_TIME as i64;
            let new_timestamp = timestamp + CurrentNetwork::ANCHOR_TIME as i64;

            let reward = coinbase_reward(
                timestamp,
                new_timestamp,
                block_height,
                CurrentNetwork::STARTING_SUPPLY,
                CurrentNetwork::ANCHOR_TIME,
            )
            .unwrap();

            assert_eq!(reward, 0);
        }
    }

    #[test]
    fn test_targets() {
        let mut rng = TestRng::default();

        let minimum_coinbase_target: u64 = 2u64.pow(10) - 1;

        fn test_new_targets<const IS_V4: bool>(rng: &mut TestRng, minimum_coinbase_target: u64) {
            let previous_coinbase_target: u64 = rng.gen_range(minimum_coinbase_target..u64::MAX);
            let previous_prover_target = proof_target(previous_coinbase_target);

            let previous_timestamp = rng.gen();

            // Targets stay the same when the timestamp is as expected.
            let new_timestamp = previous_timestamp + CurrentNetwork::ANCHOR_TIME as i64;
            let new_coinbase_target = coinbase_target::<IS_V4>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();
            let new_prover_target = proof_target(new_coinbase_target);
            assert_eq!(new_coinbase_target, previous_coinbase_target);
            assert_eq!(new_prover_target, previous_prover_target);

            // Targets decrease (easier) when the timestamp is greater than expected.
            let new_timestamp = previous_timestamp + 2 * CurrentNetwork::ANCHOR_TIME as i64;
            let new_coinbase_target = coinbase_target::<IS_V4>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();
            let new_prover_target = proof_target(new_coinbase_target);
            assert!(new_coinbase_target < previous_coinbase_target);
            assert!(new_prover_target < previous_prover_target);

            // Targets increase (harder) when the timestamp is less than expected.
            let new_timestamp = previous_timestamp + CurrentNetwork::ANCHOR_TIME as i64 / 2;
            let new_coinbase_target = coinbase_target::<IS_V4>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();
            let new_prover_target = proof_target(new_coinbase_target);

            assert!(new_coinbase_target > previous_coinbase_target);
            assert!(new_prover_target > previous_prover_target);
        }

        for _ in 0..ITERATIONS {
            test_new_targets::<true>(&mut rng, minimum_coinbase_target);
            test_new_targets::<false>(&mut rng, minimum_coinbase_target);
        }
    }

    #[test]
    fn test_target_halving() {
        let mut rng = TestRng::default();

        let minimum_coinbase_target: u64 = 2u64.pow(10) - 1;

        for _ in 0..ITERATIONS {
            let previous_coinbase_target: u64 = rng.gen_range(minimum_coinbase_target..u64::MAX);
            let previous_timestamp = rng.gen();

            let half_life = CurrentNetwork::NUM_BLOCKS_PER_EPOCH
                .saturating_div(2)
                .saturating_mul(CurrentNetwork::ANCHOR_TIME as u32) as i64;

            // New coinbase target is greater than half if the elapsed time equals the half life.
            let new_timestamp = previous_timestamp + half_life;
            let new_coinbase_target = coinbase_target::<true>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();

            assert!(new_coinbase_target > previous_coinbase_target / 2);

            // New coinbase target is halved if the elapsed time is 1 anchor time past the half life.
            let new_timestamp = previous_timestamp + half_life + CurrentNetwork::ANCHOR_TIME as i64;
            let new_coinbase_target = coinbase_target::<true>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();

            assert_eq!(new_coinbase_target, previous_coinbase_target / 2);

            // New coinbase target is less than half if the elapsed time is more than 1 anchor time past the half life.
            let new_timestamp = previous_timestamp + half_life + 2 * CurrentNetwork::ANCHOR_TIME as i64;
            let new_coinbase_target = coinbase_target::<true>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();

            assert!(new_coinbase_target < previous_coinbase_target / 2);
        }
    }

    #[test]
    fn test_target_doubling() {
        let mut rng = TestRng::default();

        // The custom block time that is faster than the anchor time.
        const BLOCK_TIME: u32 = 15;
        // The expected number of blocks before the coinbase target is doubled.
        const EXPECTED_NUM_BLOCKS_TO_DOUBLE: u32 = 321;

        let minimum_coinbase_target: u64 = 2u64.pow(10) - 1;

        let initial_coinbase_target: u64 = rng.gen_range(minimum_coinbase_target..u64::MAX / 2);
        let initial_timestamp: i64 = rng.gen();
        let mut previous_coinbase_target: u64 = initial_coinbase_target;
        let mut previous_timestamp = initial_timestamp;
        let mut num_blocks = 0;

        while previous_coinbase_target < initial_coinbase_target * 2 {
            // Targets increase (harder) when the timestamp is less than expected.
            let new_timestamp = previous_timestamp + BLOCK_TIME as i64;
            let new_coinbase_target = coinbase_target::<true>(
                previous_coinbase_target,
                previous_timestamp,
                new_timestamp,
                CurrentNetwork::ANCHOR_TIME,
                CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            )
            .unwrap();

            assert!(new_coinbase_target > previous_coinbase_target);

            previous_coinbase_target = new_coinbase_target;
            previous_timestamp = new_timestamp;
            num_blocks += 1;
        }

        println!(
            "For block times of {}s and anchor time of {}s, doubling the coinbase target took {num_blocks} blocks. ({} seconds)",
            BLOCK_TIME,
            CurrentNetwork::NUM_BLOCKS_PER_EPOCH,
            previous_timestamp - initial_timestamp
        );

        assert_eq!(EXPECTED_NUM_BLOCKS_TO_DOUBLE, num_blocks);
    }
}
