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

use snarkvm::{
    ledger::narwhal::{Transmission, TransmissionID},
    prelude::Network,
};

use indexmap::IndexMap;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Storage<N: Network> {
    /// The map of `transmission IDs` to `transmissions`.
    transmissions: Arc<RwLock<IndexMap<TransmissionID<N>, Transmission<N>>>>,
}

impl<N: Network> Default for Storage<N> {
    /// Initializes a new instance of storage.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Storage<N> {
    /// Initializes a new instance of storage.
    pub fn new() -> Self {
        Self { transmissions: Default::default() }
    }

    /// Returns `true` if the storage contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        // Check if the transmission ID exists in storage.
        self.transmissions.read().contains_key(&transmission_id.into())
    }

    /// Returns the transmission for the given `transmission ID`.
    /// If the transmission ID does not exist in storage, `None` is returned.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Transmission<N>> {
        // Get the transmission.
        self.transmissions.read().get(&transmission_id.into()).cloned()
    }

    /// Inserts the given (`transmission ID`, `transmission`) into storage.
    /// If the transmission ID already exists in storage, the existing transmission is returned.
    pub fn insert(
        &self,
        transmission_id: impl Into<TransmissionID<N>>,
        transmission: Transmission<N>,
    ) -> Option<Transmission<N>> {
        // Insert the transmission.
        self.transmissions.write().insert(transmission_id.into(), transmission)
    }
}
