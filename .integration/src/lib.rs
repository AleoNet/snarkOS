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

#![forbid(unsafe_code)]

#[cfg(feature = "test")]
pub mod client_node;
#[cfg(feature = "test")]
pub use client_node::*;

pub mod test_node;
pub use test_node::*;

#[macro_export]
macro_rules! wait_until {
    ($limit_secs: expr, $condition: expr) => {
        let now = std::time::Instant::now();
        loop {
            if $condition {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            assert!(now.elapsed() <= std::time::Duration::from_secs($limit_secs), "timed out!");
        }
    };
}
