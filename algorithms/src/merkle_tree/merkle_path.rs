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

use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::{MerkleParameters, CRH};
use snarkos_utilities::ToBytes;

pub type MerkleTreeDigest<P> = <<P as MerkleParameters>::H as CRH>::Output;

/// Stores the hashes of a particular path (in order) from leaf to root.
/// Our path `is_left_child()` if the boolean in `path` is true.
#[derive(Clone, Debug)]
pub struct MerklePath<P: MerkleParameters> {
    pub parameters: P,
    pub path: Vec<(MerkleTreeDigest<P>, MerkleTreeDigest<P>)>,
}

impl<P: MerkleParameters> MerklePath<P> {
    pub fn verify<L: ToBytes>(&self, root_hash: &MerkleTreeDigest<P>, leaf: &L) -> Result<bool, MerkleError> {
        if self.path.len() != P::DEPTH {
            return Ok(false);
        }

        // Check that the given leaf matches the leaf in the membership proof.
        if !self.path.is_empty() {
            let hash_input_size_in_bytes = (P::H::INPUT_SIZE_BITS / 8) * 2;
            let mut buffer = vec![0u8; hash_input_size_in_bytes];

            let claimed_leaf_hash = self.parameters.hash_leaf::<L>(leaf, &mut buffer)?;

            // Check if leaf is one of the bottom-most siblings.
            if claimed_leaf_hash != self.path[0].0 && claimed_leaf_hash != self.path[0].1 {
                return Ok(false);
            };

            // Check levels between leaf level and root.
            let mut previous_hash = claimed_leaf_hash;
            let mut buffer = vec![0u8; hash_input_size_in_bytes];
            for &(ref hash, ref sibling_hash) in &self.path {
                // Check if the previous hash matches the correct current hash.
                if &previous_hash != hash && &previous_hash != sibling_hash {
                    return Ok(false);
                };
                previous_hash = self.parameters.hash_inner_node(hash, sibling_hash, &mut buffer)?;
            }

            if root_hash != &previous_hash {
                return Ok(false);
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }
}

impl<P: MerkleParameters> Default for MerklePath<P> {
    fn default() -> Self {
        let mut path = Vec::with_capacity(P::DEPTH);
        for _i in 0..P::DEPTH {
            path.push((MerkleTreeDigest::<P>::default(), MerkleTreeDigest::<P>::default()));
        }
        Self {
            parameters: P::default(),
            path,
        }
    }
}
