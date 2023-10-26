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
    prelude::{Network, Result},
};

use std::fmt::Debug;

pub trait StorageService<N: Network>: Debug + Send + Sync {
    /// Stores the given `(round, transmission)` pair into storage.
    /// If the `transmission ID` already exists, the method returns an error.
    fn insert_transmission(
        &self,
        round: u64,
        transmission_id: TransmissionID<N>,
        transmission: Transmission<N>,
    ) -> Result<()>;

    /// Stores the given `(round, transmissions)` pair into storage.
    fn insert_transmissions(&self, round: u64, transmissions: Vec<(TransmissionID<N>, Transmission<N>)>) -> Result<()>;

    /// Removes the transmission for the given `round` and `transmission ID` from storage.
    fn remove_transmission(&self, round: u64, transmission_id: TransmissionID<N>) -> Result<()>;

    /// Removes the transmissions for the given `round` from storage.
    fn remove_transmissions_for_round(&self, round: u64) -> Result<()>;

    /// Returns `true` if the given `round` and `transmission ID` exist.
    fn contains_transmission(&self, round: u64, transmission_id: &TransmissionID<N>) -> Result<bool>;

    /// Returns the confirmed transmission for the given `round` and `transmission ID`.
    fn get_transmission(&self, round: u64, transmission_id: &TransmissionID<N>) -> Result<Option<Transmission<N>>>;

    /// Returns the confirmed transmission entries for the given `round`.
    fn get_transmissions(&self, round: u64) -> Result<Vec<(TransmissionID<N>, Transmission<N>)>>;
}
