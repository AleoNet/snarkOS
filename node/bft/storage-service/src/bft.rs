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
    ledger::{
        narwhal::{Transmission, TransmissionID},
        store::{BFTStorage, BFTStore},
    },
    prelude::{Network, Result},
};

use std::fmt;

/// A BFT storage service.
pub struct BFTStorageService<N: Network, B: BFTStorage<N>> {
    store: BFTStore<N, B>,
}

impl<N: Network, B: BFTStorage<N>> BFTStorageService<N, B> {
    /// Initializes a new BFT storage service.
    pub fn new(store: BFTStore<N, B>) -> Self {
        Self { store }
    }
}

impl<N: Network, B: BFTStorage<N>> fmt::Debug for BFTStorageService<N, B> {
    /// Implements a custom `fmt::Debug` for `BFTStorageService`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BFTStorageService").finish()
    }
}

impl<N: Network, B: BFTStorage<N>> StorageService<N> for BFTStorageService<N, B> {
    /// Stores the given `(round, transmission)` pair into storage.
    /// If the `transmission ID` already exists, the method returns an error.
    fn insert_transmission(
        &self,
        round: u64,
        transmission_id: TransmissionID<N>,
        transmission: Transmission<N>,
    ) -> Result<()> {
        self.store.insert_transmission(round, transmission_id, transmission)
    }

    /// Stores the given `(round, transmissions)` pair into storage.
    fn insert_transmissions(&self, round: u64, transmissions: Vec<(TransmissionID<N>, Transmission<N>)>) -> Result<()> {
        self.store.insert_transmissions(round, transmissions)
    }

    /// Removes the transmission for the given `round` and `transmission ID` from storage.
    fn remove_transmission(&self, round: u64, transmission_id: TransmissionID<N>) -> Result<()> {
        self.store.remove_transmission(round, transmission_id)
    }

    /// Removes the transmissions for the given `round` from storage.
    fn remove_transmissions_for_round(&self, round: u64) -> Result<()> {
        self.store.remove_transmissions_for_round(round)
    }

    /// Returns `true` if the given `round` and `transmission ID` exist.
    fn contains_transmission(&self, round: u64, transmission_id: &TransmissionID<N>) -> Result<bool> {
        self.store.contains_transmission_confirmed(round, transmission_id)
    }

    /// Returns the confirmed transmission for the given `round` and `transmission ID`.
    fn get_transmission(&self, round: u64, transmission_id: &TransmissionID<N>) -> Result<Option<Transmission<N>>> {
        self.store.get_transmission_confirmed(round, transmission_id)
    }

    /// Returns the confirmed transmission entries for the given `round`.
    fn get_transmissions(&self, round: u64) -> Result<Vec<(TransmissionID<N>, Transmission<N>)>> {
        self.store.get_transmissions_confirmed(round)
    }
}
