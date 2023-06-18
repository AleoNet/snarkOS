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

use crate::helpers::TransmissionID;
use snarkvm::console::prelude::*;

use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct Pending<N: Network> {
    /// The map of pending `transmission IDs` to `peer IPs` that have the transmission.
    transmissions: Arc<RwLock<HashMap<TransmissionID<N>, HashSet<SocketAddr>>>>,
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
        Self { transmissions: Default::default() }
    }

    /// Returns the number of transmissions in the pending queue.
    pub fn len(&self) -> usize {
        self.transmissions.read().len()
    }

    /// Returns `true` if the pending queue contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmissions.read().contains_key(&transmission_id.into())
    }

    /// Returns `true` if the pending queue contains the specified `transmission ID` for the specified `peer IP`.
    pub fn contains_peer(&self, transmission_id: impl Into<TransmissionID<N>>, peer_ip: SocketAddr) -> bool {
        self.transmissions.read().get(&transmission_id.into()).map_or(false, |peer_ips| peer_ips.contains(&peer_ip))
    }

    /// Returns the peer IPs for the specified `transmission ID`.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<HashSet<SocketAddr>> {
        self.transmissions.read().get(&transmission_id.into()).cloned()
    }

    /// Inserts the specified `transmission ID` and `peer IP` to the pending queue.
    /// If the `transmission ID` already exists, the `peer IP` is added to the existing transmission.
    pub fn insert(&self, transmission_id: impl Into<TransmissionID<N>>, peer_ip: SocketAddr) {
        self.transmissions.write().entry(transmission_id.into()).or_default().insert(peer_ip);
    }

    /// Removes the specified `transmission ID` from the pending queue.
    pub fn remove(&self, transmission_id: impl Into<TransmissionID<N>>) {
        self.transmissions.write().remove(&transmission_id.into());
    }
}
