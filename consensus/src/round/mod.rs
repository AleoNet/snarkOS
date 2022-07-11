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

use crate::Status;
use snarkvm::prelude::{Address, Network};

#[derive(Clone, Debug)]
pub struct Round<N: Network> {
    /// The number of rounds since the genesis round.
    id: u64,
    /// The status of the round.
    status: Status,
    /// The leader of the round.
    leader: Address<N>,
}

impl<N: Network> Round<N> {
    /// Initializes a new round, given the round ID and leader address.
    pub const fn new(id: u64, leader: Address<N>) -> Self {
        Self {
            id,
            status: Status::Running,
            leader,
        }
    }

    /// Returns the round number.
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Returns the status of the round.
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Returns the leader of the round.
    pub const fn leader(&self) -> &Address<N> {
        &self.leader
    }
}
