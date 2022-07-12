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

pub mod block_state;
pub mod ledger_state;
pub mod transaction_state;

use snarkvm::prelude::*;

///
/// A helper struct containing transaction metadata.
///
/// *Attention*: This data structure is intended for usage in storage only.
/// Modifications to its layout will impact how metadata is represented in storage.
///
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Metadata<N: Network> {
    block_height: u32,
    block_hash: N::BlockHash,
    block_timestamp: i64,
    transaction_index: u16,
}

impl<N: Network> Metadata<N> {
    /// Initializes a new instance of `Metadata`.
    pub fn new(block_height: u32, block_hash: N::BlockHash, block_timestamp: i64, transaction_index: u16) -> Self {
        Self {
            block_height,
            block_hash,
            block_timestamp,
            transaction_index,
        }
    }
}
