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

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use parking_lot::RwLock;

use crate::Stats;

/// Contains statistics related to Tcp's peers, currently connected or not.
#[derive(Default)]
pub struct KnownPeers(RwLock<HashMap<SocketAddr, Arc<Stats>>>);

impl KnownPeers {
    /// Adds an address to the list of known peers.
    pub fn add(&self, addr: SocketAddr) {
        self.0.write().entry(addr).or_default();
    }

    /// Returns the stats for the given peer.
    pub fn get(&self, addr: SocketAddr) -> Option<Arc<Stats>> {
        self.0.read().get(&addr).map(Arc::clone)
    }

    /// Removes an address from the list of known peers.
    pub fn remove(&self, addr: SocketAddr) -> Option<Arc<Stats>> {
        self.0.write().remove(&addr)
    }

    /// Returns the list of all known peers and their stats.
    pub fn snapshot(&self) -> HashMap<SocketAddr, Arc<Stats>> {
        self.0.read().clone()
    }

    /// Registers a submission of a message to the given address.
    pub fn register_sent_message(&self, to: SocketAddr, size: usize) {
        if let Some(stats) = self.0.read().get(&to) {
            stats.register_sent_message(size);
        }
    }

    /// Registers a receipt of a message to the given address.
    pub fn register_received_message(&self, from: SocketAddr, size: usize) {
        if let Some(stats) = self.0.read().get(&from) {
            stats.register_received_message(size);
        }
    }

    /// Registers a failure associated with the given address.
    pub fn register_failure(&self, addr: SocketAddr) {
        if let Some(stats) = self.0.read().get(&addr) {
            stats.register_failure();
        }
    }
}
