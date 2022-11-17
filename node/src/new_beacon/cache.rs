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

use crate::new_beacon::circular_map::CircularMap;
use parking_lot::RwLock;
use snarkvm::prelude::Network as CurrentNetwork;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone)]
pub struct Cache<N: CurrentNetwork> {
    seen_inbound_blocks: Arc<RwLock<CircularMap<N::BlockHash, OffsetDateTime, 256>>>,
}

impl<N: CurrentNetwork> Cache<N> {
    pub fn new() -> Self {
        Self { seen_inbound_blocks: Arc::new(RwLock::new(CircularMap::new())) }
    }

    pub fn insert_seen_block(&self, hash: N::BlockHash) -> bool {
        self.seen_inbound_blocks.write().insert(hash, OffsetDateTime::now_utc())
    }
}
