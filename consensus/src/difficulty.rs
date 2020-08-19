// Copyright (C) 2019-2020 Aleo Systems Inc.
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

pub const DIFFICULTY_BOMB_DIVISOR: f64 = 1024f64;

/// Linear difficulty recalculation using time elapsed.
pub fn naive_retarget(
    block_timestamp: i64,
    parent_timestamp: i64,
    target_block_time: i64,
    parent_difficulty: u64,
) -> u64 {
    let time_elapsed = block_timestamp - parent_timestamp;
    if time_elapsed == target_block_time || time_elapsed == 0 {
        parent_difficulty
    } else {
        let parent_diff = parent_difficulty as f64;
        let mut x: f64;

        // (target_block_time - time_elapsed) / target_block_time
        x = (target_block_time - time_elapsed) as f64;
        x /= target_block_time as f64;

        // parent_diff - ((target_block_time - block_time) / target_block_time * parent_diff)
        x *= parent_diff;
        x = parent_diff - x;

        println!("old difficulty        {:#x}", parent_difficulty);
        println!("new difficulty        {:#x}", x as u64);

        x as u64
    }
}

/// Bitcoin difficulty retarget algorithm.
pub fn bitcoin_retarget(
    block_timestamp: i64,
    parent_timestamp: i64,
    target_block_time: i64,
    parent_difficulty: u64,
) -> u64 {
    let mut time_elapsed = block_timestamp - parent_timestamp;

    // Limit difficulty adjustment by factor of 2
    if time_elapsed < target_block_time / 2 {
        time_elapsed = target_block_time / 2
    } else if time_elapsed > target_block_time * 2 {
        time_elapsed = target_block_time * 2
    }

    let mut x: u64;
    x = match parent_difficulty.checked_mul(time_elapsed as u64) {
        Some(x) => x,
        None => u64::max_value(),
    };

    x /= target_block_time as u64;

    x
}

/// Custom difficulty retarget algorithm.
pub fn custom_retarget(
    _block_timestamp: i64,
    _parent_timestamp: i64,
    _target_block_time: i64,
    _parent_difficulty: u64,
) -> u64 {
    unimplemented!()
}

/// Ethereum difficulty retarget algorithm.
pub fn ethereum_retarget(block_timestamp: i64, parent_timestamp: i64, parent_difficulty: u64) -> u64 {
    let parent_diff = parent_difficulty as f64;
    let mut x: f64;
    let y: f64;

    // 1 - (block_timestamp - parent_timestamp) // 10
    x = (block_timestamp - parent_timestamp) as f64;
    x /= 10f64;
    x = 1f64 - x;

    // max (1 - (block_timestamp - parent_timestamp) // 10, -99))
    x = f64::max(x, -99f64);

    // (parent_diff + parent_diff // 2048 * max(1 - (block_timestamp - parent_timestamp) // 10, -99))
    y = parent_diff / DIFFICULTY_BOMB_DIVISOR;
    x *= y;
    x += parent_diff;

    println!("old difficulty        {:#x}", parent_difficulty);
    println!("new difficulty        {:#x}", x as u64);

    x as u64
}
