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

use crate::Digest;

#[derive(Clone, Debug)]
pub enum DigestTree {
    // digest of leaf node
    Leaf(Digest),
    // digest and subtree of node, length of longest chain not including node
    Node(Digest, Vec<DigestTree>, usize),
}

impl DigestTree {
    pub fn with_children(self, children: Vec<Digest>) -> Self {
        let digest = match self {
            DigestTree::Leaf(digest) => digest,
            _ => panic!("cannot add children to non-leaf node"),
        };

        if children.is_empty() {
            DigestTree::Leaf(digest)
        } else {
            DigestTree::Node(
                digest,
                children.into_iter().map(|child| DigestTree::Leaf(child)).collect(),
                2,
            )
        }
    }

    pub fn root(&self) -> &Digest {
        match self {
            DigestTree::Leaf(root) => root,
            DigestTree::Node(root, _, _) => root,
        }
    }

    pub fn longest_length(&self) -> usize {
        match self {
            DigestTree::Leaf(_) => 1,
            DigestTree::Node(_, _, n) => *n + 1,
        }
    }

    pub fn unified_chain(&self) -> Option<Vec<&Digest>> {
        let mut out = vec![];
        let mut current_node = self;
        loop {
            match current_node {
                DigestTree::Leaf(hash) => {
                    out.push(hash);
                    break;
                }
                DigestTree::Node(hash, children, _) => {
                    if children.len() != 1 {
                        return None;
                    }
                    out.push(hash);
                    current_node = &children[0];
                }
            }
        }
        Some(out)
    }

    pub fn children(&self) -> &[DigestTree] {
        match self {
            DigestTree::Leaf(_) => &[],
            DigestTree::Node(_, children, _) => &children[..],
        }
    }
}
