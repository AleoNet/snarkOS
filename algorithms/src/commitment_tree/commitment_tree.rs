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

use crate::commitment_tree::CommitmentMerklePath;
use snarkos_errors::algorithms::MerkleError;
use snarkos_models::algorithms::{CommitmentScheme, CRH};
use snarkos_utilities::{to_bytes, FromBytes, ToBytes};

use std::io::{Read, Result as IoResult, Write};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: CommitmentScheme, H: CRH"),
    PartialEq(bound = "C: CommitmentScheme, H: CRH"),
    Eq(bound = "C: CommitmentScheme, H: CRH")
)]
pub struct CommitmentMerkleTree<C: CommitmentScheme, H: CRH> {
    /// The computed root of the full Merkle tree.
    root: <H as CRH>::Output,

    /// The internal hashes of the commitment Merkle tree
    inner_hashes: (<H as CRH>::Output, <H as CRH>::Output),

    /// The leaves of the commitment Merkle tree
    leaves: [<C as CommitmentScheme>::Output; 4],

    /// The CRH parameters used to construct the Merkle tree
    #[derivative(PartialEq = "ignore")]
    parameters: H,
}

impl<C: CommitmentScheme, H: CRH> CommitmentMerkleTree<C, H> {
    /// Construct a new commitment Merkle tree.
    pub fn new(parameters: H, leaves: &[<C as CommitmentScheme>::Output; 4]) -> Result<Self, MerkleError> {
        let input_1 = to_bytes![leaves[0], leaves[1]]?;
        let inner_hash1 = H::hash(&parameters, &input_1)?;

        let input_2 = to_bytes![leaves[2], leaves[3]]?;
        let inner_hash2 = H::hash(&parameters, &input_2)?;

        let root = H::hash(&parameters, &to_bytes![inner_hash1, inner_hash2]?)?;

        Ok(Self {
            root,
            inner_hashes: (inner_hash1, inner_hash2),
            leaves: leaves.clone(),
            parameters,
        })
    }

    #[inline]
    pub fn root(&self) -> <H as CRH>::Output {
        self.root.clone()
    }

    #[inline]
    pub fn inner_hashes(&self) -> (<H as CRH>::Output, <H as CRH>::Output) {
        self.inner_hashes.clone()
    }

    #[inline]
    pub fn leaves(&self) -> [<C as CommitmentScheme>::Output; 4] {
        self.leaves.clone()
    }

    pub fn generate_proof(
        &self,
        leaf: &<C as CommitmentScheme>::Output,
    ) -> Result<CommitmentMerklePath<C, H>, MerkleError> {
        let leaf_index = match self.leaves.iter().position(|l| l == leaf) {
            Some(index) => index,
            _ => return Err(MerkleError::InvalidLeaf),
        };

        let sibling_index = sibling(leaf_index);

        let leaf = leaf.clone();
        let sibling = self.leaves[sibling_index].clone();

        let leaves = match is_left_child(leaf_index) {
            true => (leaf, sibling),
            false => (sibling, leaf),
        };

        let inner_hashes = self.inner_hashes.clone();

        Ok(CommitmentMerklePath { leaves, inner_hashes })
    }

    pub fn from_bytes<R: Read>(mut reader: R, parameters: H) -> IoResult<Self> {
        let root = <H as CRH>::Output::read(&mut reader)?;

        let left_inner_hash = <H as CRH>::Output::read(&mut reader)?;
        let right_inner_hash = <H as CRH>::Output::read(&mut reader)?;

        let inner_hashes = (left_inner_hash, right_inner_hash);

        let mut leaves = vec![];
        for _ in 0..4 {
            let leaf = <C as CommitmentScheme>::Output::read(&mut reader)?;
            leaves.push(leaf);
        }

        assert_eq!(leaves.len(), 4);

        let leaves = [
            leaves[0].clone(),
            leaves[1].clone(),
            leaves[2].clone(),
            leaves[3].clone(),
        ];

        Ok(Self {
            root,
            inner_hashes,
            leaves,
            parameters,
        })
    }
}

impl<C: CommitmentScheme, H: CRH> ToBytes for CommitmentMerkleTree<C, H> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.root.write(&mut writer)?;
        self.inner_hashes.0.write(&mut writer)?;
        self.inner_hashes.1.write(&mut writer)?;
        for leaf in &self.leaves {
            leaf.write(&mut writer)?;
        }

        Ok(())
    }
}

/// Returns the index of the sibling leaf, given an index.
#[inline]
fn sibling(index: usize) -> usize {
    assert!(index < 4);
    match index {
        0 => 1,
        1 => 0,
        2 => 3,
        3 => 2,
        _ => unreachable!(),
    }
}

/// Returns true iff the given index represents a left child.
#[inline]
fn is_left_child(index: usize) -> bool {
    index % 2 == 0
}
