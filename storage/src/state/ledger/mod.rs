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

pub(super) mod block_state;
// pub(super) mod ledger_state;
pub(super) mod transaction_state;

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

#[cfg(test)]
pub(crate) mod test_helpers {
    use snarkvm::{
        console::{account::PrivateKey, network::Testnet3},
        utilities::test_crypto_rng_fixed,
        Block,
        VM,
    };

    use once_cell::sync::OnceCell;

    pub(crate) type CurrentNetwork = Testnet3;

    pub(crate) fn sample_genesis_block() -> Block<CurrentNetwork> {
        static INSTANCE: OnceCell<Block<CurrentNetwork>> = OnceCell::new();
        INSTANCE
            .get_or_init(|| {
                // Initialize the VM.
                let mut vm = VM::<CurrentNetwork>::new().unwrap();
                // Initialize the RNG.
                let rng = &mut test_crypto_rng_fixed();
                // Initialize a new caller.
                let caller_private_key = PrivateKey::<CurrentNetwork>::new(rng).unwrap();
                // Return the block.
                Block::genesis(&mut vm, &caller_private_key, rng).unwrap()
            })
            .clone()
    }
}
