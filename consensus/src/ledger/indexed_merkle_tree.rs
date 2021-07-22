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

use std::sync::Arc;

use crate::IndexedDigests;
use anyhow::*;
use snarkos_storage::Digest;
use snarkvm_algorithms::{merkle_tree::MerkleTree, MerkleParameters};
use snarkvm_utilities::ToBytes;

pub struct IndexedMerkleTree<P: MerkleParameters> {
    tree: MerkleTree<P>,
    indexed_digests: IndexedDigests,
}

impl<P: MerkleParameters> Clone for IndexedMerkleTree<P> {
    fn clone(&self) -> Self {
        let tree = self
            .tree
            .rebuild::<[u8; 32]>(self.indexed_digests.len(), &[])
            .expect("failed to clone merkle tree");
        Self {
            tree,
            indexed_digests: self.indexed_digests.clone(),
        }
    }
}

fn to_digest<B: ToBytes>(input: &B) -> Result<Digest> {
    let mut data = vec![];
    input.write_le(&mut data)?;
    Ok((&data[..]).into())
}

impl<P: MerkleParameters> IndexedMerkleTree<P> {
    pub fn new(parameters: Arc<P>, leaves: &[Digest]) -> Result<Self> {
        Ok(Self {
            tree: MerkleTree::new(parameters, leaves)?,
            indexed_digests: IndexedDigests::new(leaves),
        })
    }

    pub fn extend(&mut self, new_leaves: &[Digest]) -> Result<()> {
        let tree = self.tree.rebuild(self.indexed_digests.len(), new_leaves)?;
        self.tree = tree;
        self.indexed_digests.extend(new_leaves);
        Ok(())
    }

    /// pop leafs from the interior merkle tree, and assert they are equal to `to_remove`.
    pub fn pop(&mut self, to_remove: &[Digest]) -> Result<()> {
        if to_remove.len() > self.indexed_digests.len() {
            return Err(anyhow!(
                "attempted to remove more items from indexed merkle tree than present"
            ));
        }
        self.indexed_digests.pop(to_remove)?;
        let tree = self.tree.rebuild::<[u8; 32]>(self.indexed_digests.len(), &[])?;
        self.tree = tree;

        Ok(())
    }

    pub fn clear(&mut self) {
        self.indexed_digests.clear();
        self.tree = self.tree.rebuild::<[u8; 32]>(0, &[]).unwrap();
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
        self.indexed_digests.index(leaf)
    }

    pub fn digest(&self) -> Digest {
        let mut out = vec![];
        self.tree.root().write_le(&mut out).expect("failed to digest root");
        (&out[..]).into()
    }

    pub fn generate_proof(&self, commitment: &Digest, index: usize) -> Result<Vec<(Digest, Digest)>> {
        let path = self.tree.generate_proof(index, commitment)?;
        path.path
            .into_iter()
            .map(|(p1, p2)| Ok((to_digest(&p1)?, to_digest(&p2)?)))
            .collect::<Result<Vec<_>>>()
    }
}
