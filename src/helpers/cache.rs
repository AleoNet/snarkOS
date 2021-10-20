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

use snarkvm::parameters::{testnet2::GenesisBlockHeader, Genesis};

use circular_queue::CircularQueue;
use twox_hash::xxh3::hash64;

// TODO (howardwu): CRITICAL - FIX this legacy mess.
const BLOCK_HEADER_SIZE: usize = GenesisBlockHeader::SIZE as usize;
const PEER_BLOCK_CACHE_SIZE: usize = 8;

pub type BlockCache = Cache<PEER_BLOCK_CACHE_SIZE, BLOCK_HEADER_SIZE>;

#[derive(Debug, Clone)]
pub struct Cache<const N: usize, const S: usize> {
    queue: CircularQueue<u64>,
}

impl<const N: usize, const S: usize> Default for Cache<N, S> {
    fn default() -> Self {
        Self {
            queue: CircularQueue::with_capacity(N),
        }
    }
}

impl<const N: usize, const S: usize> Cache<N, S> {
    fn hash_block(payload: &[u8]) -> u64 {
        hash64(&payload[..S.min(payload.len())])
    }

    pub fn contains(&self, payload: &[u8]) -> bool {
        let hash = Self::hash_block(payload);

        self.queue.iter().any(|&e| e == hash)
    }

    pub fn push(&mut self, payload: &[u8]) {
        let hash = Self::hash_block(payload);

        self.queue.push(hash);
    }
}
