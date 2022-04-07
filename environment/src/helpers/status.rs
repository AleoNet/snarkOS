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

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    },
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[repr(u8)]
pub enum Status {
    /// The ledger is ready to handle requests.
    Ready = 0,
    /// The ledger is mining the next block.
    Mining,
    /// The ledger is connecting to the minimum number of required peers.
    Peering,
    /// The ledger is syncing blocks with a connected peer.
    Syncing,
    /// The ledger is terminating and shutting down.
    ShuttingDown,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug)]
pub struct RawStatus(Arc<AtomicU8>);

impl RawStatus {
    /// Initializes a new instance of `RawStatus`.
    pub fn new() -> Self {
        Self(Arc::new(AtomicU8::new(Status::Peering as u8)))
    }

    /// Updates the status to the given value.
    pub fn update(&self, status: Status) {
        self.0.store(status as u8, Ordering::SeqCst);
    }

    /// Returns the status of the node.
    pub fn get(&self) -> Status {
        match self.0.load(Ordering::SeqCst) {
            0 => Status::Ready,
            1 => Status::Mining,
            2 => Status::Peering,
            3 => Status::Syncing,
            4 => Status::ShuttingDown,
            _ => unreachable!("Invalid status code"),
        }
    }

    /// Returns `true` if the node is ready to handle requests.
    pub fn is_ready(&self) -> bool {
        self.get() == Status::Ready
    }

    /// Returns `true` if the node is currently mining.
    pub fn is_mining(&self) -> bool {
        self.get() == Status::Mining
    }

    /// Returns `true` if the node is currently peering.
    pub fn is_peering(&self) -> bool {
        self.get() == Status::Peering
    }

    /// Returns `true` if the node is currently syncing.
    pub fn is_syncing(&self) -> bool {
        self.get() == Status::Syncing
    }
}

impl fmt::Display for RawStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.get())
    }
}

impl Default for RawStatus {
    fn default() -> Self {
        Self::new()
    }
}
