// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{bytes_to_u32, Ledger, COL_BLOCK_LOCATOR};
use snarkvm_algorithms::traits::LoadableMerkleParameters;
use snarkvm_dpc::{BlockHeaderHash, Storage, TransactionScheme};
use snarkvm_utilities::ToBytes;

use tracing::*;

use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    fs,
    hash::{Hash, Hasher},
    path::Path,
};

struct BlockLocatorPair(Box<[u8]>, Box<[u8]>);

impl PartialEq for BlockLocatorPair {
    fn eq(&self, other: &Self) -> bool {
        if self.0.len() == other.0.len() {
            // (key0, value0) == (key1, value1) || (value0, key0) == (value1, key1)
            self.0 == other.0
        } else {
            // (key0, value0) == (value1, key1) || (value0, key0) == (key1, value1)
            self.0 == other.1
        }
    }
}
impl Eq for BlockLocatorPair {}

impl Hash for BlockLocatorPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Just hash the block number, it's unique
        if self.0.len() < self.1.len() {
            // (number, hash)
            self.0.hash(state);
        } else {
            // (hash, number)
            self.1.hash(state);
        }
    }
}

impl Ord for BlockLocatorPair {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0.len() == other.0.len() {
            // (key0, value0) == (key1, value1) || (value0, key0) == (value1, key1)
            self.0.cmp(&other.0)
        } else {
            // (key0, value0) == (value1, key1) || (value0, key0) == (key1, value1)
            self.0.cmp(&other.1)
        }
    }
}

impl PartialOrd for BlockLocatorPair {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: TransactionScheme, P: LoadableMerkleParameters, S: Storage> Ledger<T, P, S> {
    pub fn export_canon_blocks(&self, limit: u32, location: &Path) -> Result<(), anyhow::Error> {
        info!("Exporting node's canon blocks to {}", location.display());

        let locator_col = self.storage.get_col(COL_BLOCK_LOCATOR)?;

        let numbers_and_hashes = locator_col
            .into_iter()
            .filter(|(locator_key, locator_value)| locator_key.len() < locator_value.len())
            .map(|(block_number_bytes, block_hash)| (bytes_to_u32(&block_number_bytes), block_hash))
            .collect::<BinaryHeap<_>>()
            .into_sorted_vec();

        let number_to_export = if limit == 0 { u32::MAX } else { limit } as usize;

        let mut serialized_blocks = Vec::new();
        // Skip the genesis block, as it's always known.
        for (_block_number, block_hash) in numbers_and_hashes.into_iter().skip(1).take(number_to_export) {
            let block = self.get_block(&BlockHeaderHash::new(block_hash.into_vec()))?;
            block.write(&mut serialized_blocks)?;
        }

        fs::write(location, serialized_blocks)?;

        Ok(())
    }
}
