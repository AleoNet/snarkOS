// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use snarkvm::prelude::Network;

use core::hash::Hash;
use indexmap::IndexSet;
use std::net::SocketAddr;

/// A tuple of the block hash (optional), previous block hash (optional), and the number of sync IPS to request from.
pub type PrepareSyncRequest<N> = (Option<<N as Network>::BlockHash>, Option<<N as Network>::BlockHash>, usize);

/// A tuple of the block hash (optional), previous block hash (optional), and sync IPs.
pub type SyncRequest<N> = (Option<<N as Network>::BlockHash>, Option<<N as Network>::BlockHash>, IndexSet<SocketAddr>);

#[derive(Copy, Clone, Debug)]
pub(crate) struct PeerPair(pub SocketAddr, pub SocketAddr);

impl Eq for PeerPair {}

impl PartialEq for PeerPair {
    fn eq(&self, other: &Self) -> bool {
        (self.0 == other.0 && self.1 == other.1) || (self.0 == other.1 && self.1 == other.0)
    }
}

impl Hash for PeerPair {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (a, b) = if self.0 < self.1 { (self.0, self.1) } else { (self.1, self.0) };
        a.hash(state);
        b.hash(state);
    }
}
