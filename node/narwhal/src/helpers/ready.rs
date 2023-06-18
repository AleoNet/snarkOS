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

use crate::helpers::{Transmission, TransmissionID};
use snarkos_node_messages::Data;
use snarkvm::console::prelude::*;

use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct Ready<N: Network> {
    /// The map of `transmission IDs` to `transmissions`.
    transmissions: Arc<RwLock<HashMap<TransmissionID<N>, Data<Transmission<N>>>>>,
}

impl<N: Network> Default for Ready<N> {
    /// Initializes a new instance of the ready queue.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Ready<N> {
    /// Initializes a new instance of the ready queue.
    pub fn new() -> Self {
        Self { transmissions: Default::default() }
    }

    /// Returns the transmissions.
    pub const fn transmissions(&self) -> &Arc<RwLock<HashMap<TransmissionID<N>, Data<Transmission<N>>>>> {
        &self.transmissions
    }

    /// Returns the number of transmissions in the ready queue.
    pub fn len(&self) -> usize {
        self.transmissions.read().len()
    }

    /// Returns the transmission IDs.
    pub fn transmission_ids(&self) -> Vec<TransmissionID<N>> {
        self.transmissions.read().keys().copied().collect()
    }

    /// Returns `true` if the ready queue contains the specified `transmission ID`.
    pub fn contains(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool {
        self.transmissions.read().contains_key(&transmission_id.into())
    }

    /// Returns the transmission, given the specified `transmission ID`.
    pub fn get(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Data<Transmission<N>>> {
        self.transmissions.read().get(&transmission_id.into()).cloned()
    }

    /// Inserts the specified (`transmission ID`, `transmission`) to the ready queue.
    pub fn insert(&self, transmission_id: impl Into<TransmissionID<N>>, transmission: Data<Transmission<N>>) {
        self.transmissions.write().insert(transmission_id.into(), transmission);
    }

    /// Removes the specified `transmission ID` from the ready queue.
    pub fn remove(&self, transmission_id: impl Into<TransmissionID<N>>) {
        self.transmissions.write().remove(&transmission_id.into());
    }

    /// Removes the transmissions and returns them.
    pub fn drain(&self) -> HashMap<TransmissionID<N>, Data<Transmission<N>>> {
        self.transmissions.write().drain().map(|(k, v)| (k, v)).collect()
    }
}
