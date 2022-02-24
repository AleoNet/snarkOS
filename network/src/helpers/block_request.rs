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

use snarkvm::dpc::prelude::*;

use std::hash::{Hash, Hasher};

///
/// A request for a block with the specified height and an optional hash.
///
#[derive(Clone, Debug)]
pub struct BlockRequest<N: Network> {
    block_height: u32,
    block_hash: Option<N::BlockHash>,
}

impl<N: Network> BlockRequest<N> {
    /// Returns the block height stored in the request.
    pub fn block_height(&self) -> u32 {
        self.block_height
    }

    /// Returns the block hash stored in the request, if it exists.
    pub fn block_hash(&self) -> Option<N::BlockHash> {
        self.block_hash
    }
}

impl<N: Network> From<u32> for BlockRequest<N> {
    fn from(height: u32) -> Self {
        Self {
            block_height: height,
            block_hash: None,
        }
    }
}

impl<N: Network> From<(u32, Option<N::BlockHash>)> for BlockRequest<N> {
    fn from((height, hash): (u32, Option<N::BlockHash>)) -> Self {
        Self {
            block_height: height,
            block_hash: hash,
        }
    }
}

// The height is the primary key, so use only it for hashing purposes.
impl<N: Network> PartialEq for BlockRequest<N> {
    fn eq(&self, other: &Self) -> bool {
        self.block_height == other.block_height
    }
}

impl<N: Network> Eq for BlockRequest<N> {}

// The k1 == k2 -> hash(k1) == hash(k2) rule must hold.
impl<N: Network> Hash for BlockRequest<N> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.block_height.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::{dpc::testnet2::Testnet2, prelude::UniformRand};

    use rand::{thread_rng, Rng};

    #[test]
    fn test_block_request() {
        let rng = &mut thread_rng();

        for _ in 0..5 {
            let block_height: u32 = rng.gen();
            let block_hash = <Testnet2 as Network>::BlockHash::rand(rng);

            let request = BlockRequest::<Testnet2>::from(block_height);
            assert_eq!(block_height, request.block_height());
            assert_eq!(None, request.block_hash());

            let request = BlockRequest::<Testnet2>::from((block_height, None));
            assert_eq!(block_height, request.block_height());
            assert_eq!(None, request.block_hash());

            let request = BlockRequest::<Testnet2>::from((block_height, Some(block_hash)));
            assert_eq!(block_height, request.block_height());
            assert_eq!(Some(block_hash), request.block_hash());
        }
    }

    #[test]
    fn test_block_request_eq() {
        let rng = &mut thread_rng();

        for _ in 0..5 {
            let block_height: u32 = rng.gen();
            let block_hash = <Testnet2 as Network>::BlockHash::rand(rng);

            let a = BlockRequest::<Testnet2>::from(block_height);
            let b = BlockRequest::<Testnet2>::from((block_height, None));
            assert_eq!(a, b);

            let a = BlockRequest::<Testnet2>::from(block_height);
            let b = BlockRequest::<Testnet2>::from((block_height, Some(block_hash)));
            assert_eq!(a, b);
        }
    }
}
