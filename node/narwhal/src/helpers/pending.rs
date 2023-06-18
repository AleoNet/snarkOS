// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use crate::helpers::EntryID;
use snarkvm::console::prelude::*;

use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Pending<N: Network> {
    /// The map of pending `entry IDs` to `peer IPs` that have the entry.
    entries: Arc<RwLock<HashMap<EntryID<N>, HashSet<SocketAddr>>>>,
}

impl<N: Network> Default for Pending<N> {
    /// Initializes a new instance of the pending queue.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Pending<N> {
    /// Initializes a new instance of the pending queue.
    pub fn new() -> Self {
        Self { entries: Default::default() }
    }

    /// Returns the number of entries in the pending queue.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Returns `true` if the pending queue contains the specified `entry ID`.
    pub fn contains(&self, entry_id: impl Into<EntryID<N>>) -> bool {
        self.entries.read().contains_key(&entry_id.into())
    }

    /// Returns `true` if the pending queue contains the specified `entry ID` for the specified `peer IP`.
    pub fn contains_peer(&self, entry_id: impl Into<EntryID<N>>, peer_ip: SocketAddr) -> bool {
        self.entries.read().get(&entry_id.into()).map_or(false, |peer_ips| peer_ips.contains(&peer_ip))
    }

    /// Inserts the specified `entry ID` and `peer IP` to the pending queue.
    /// If the `entry ID` already exists, the `peer IP` is added to the existing entry.
    pub fn insert(&self, entry_id: impl Into<EntryID<N>>, peer_ip: SocketAddr) {
        self.entries.write().entry(entry_id.into()).or_default().insert(peer_ip);
    }

    /// Removes the specified `entry ID` from the pending queue.
    pub fn remove(&self, entry_id: impl Into<EntryID<N>>) {
        self.entries.write().remove(&entry_id.into());
    }
}
