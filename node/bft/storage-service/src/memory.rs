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

use crate::StorageService;
use snarkvm::{
    ledger::narwhal::{Data, Transmission, TransmissionID},
    prelude::{bail, Field, Network, Result},
};

use indexmap::{IndexMap, IndexSet};
use parking_lot::RwLock;

/// A BFT in-memory storage service.
#[derive(Debug)]
pub struct BFTMemoryService<N: Network> {
    /// The map of `transmission ID` to `(transmission, certificate IDs)` entries.
    transmissions: RwLock<IndexMap<TransmissionID<N>, (Transmission<N>, IndexSet<Field<N>>)>>,
}

impl<N: Network> Default for BFTMemoryService<N> {
    /// Initializes a new BFT in-memory storage service.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> BFTMemoryService<N> {
    /// Initializes a new BFT in-memory storage service.
    pub fn new() -> Self {
        Self { transmissions: Default::default() }
    }
}

impl<N: Network> StorageService<N> for BFTMemoryService<N> {}
