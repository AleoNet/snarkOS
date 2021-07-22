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

use anyhow::*;
use indexmap::IndexSet;
use snarkos_storage::Digest;

#[derive(Clone)]
pub struct IndexedDigests {
    indexed_digests: IndexSet<Digest>,
}

impl IndexedDigests {
    pub fn new(leaves: &[Digest]) -> Self {
        Self {
            indexed_digests: leaves.iter().cloned().collect(),
        }
    }

    pub fn extend(&mut self, new_leaves: &[Digest]) {
        self.indexed_digests.extend(new_leaves.iter().cloned());
    }

    /// pop leafs from the interior merkle tree, and assert they are equal to `to_remove`.
    pub fn pop(&mut self, to_remove: &[Digest]) -> Result<()> {
        if to_remove.len() > self.indexed_digests.len() {
            return Err(anyhow!(
                "attempted to remove more items from indexed digests set than present"
            ));
        }
        let old_length = self.indexed_digests.len() - to_remove.len();
        for i in old_length..self.indexed_digests.len() {
            if self.indexed_digests[i] != to_remove[i - old_length] {
                return Err(anyhow!(
                    "mismatch in attempted pop of indexed digests @ {}: {} != {}",
                    i,
                    self.indexed_digests[i],
                    to_remove[i - old_length]
                ));
            }
        }
        self.indexed_digests.truncate(old_length);

        Ok(())
    }

    pub fn clear(&mut self) {
        self.indexed_digests.clear();
    }

    pub fn len(&self) -> usize {
        self.indexed_digests.len()
    }

    pub fn is_empty(&self) -> bool {
        self.indexed_digests.is_empty()
    }

    pub fn contains(&self, leaf: &Digest) -> bool {
        self.indexed_digests.contains(leaf)
    }

    pub fn index(&self, leaf: &Digest) -> Option<usize> {
        self.indexed_digests.get_index_of(leaf)
    }
}
