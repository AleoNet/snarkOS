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
    /// Stores the given round, transmission ID, and transmission into storage.
    fn insert_transmission(
        &self,
        round: u64,
        transmission_id: TransmissionID<N>,
        transmission: Transmission<N>,
    ) -> Result<()>;

    /// Stores the given `(transmission ID, transmission)` pairs for the given round into storage.
    fn insert_transmissions(&self, round: u64, transmissions: Vec<(TransmissionID<N>, Transmission<N>)>) -> Result<()>;

    /// Removes the transmission for the given `transmission ID` from storage.
    fn remove_transmission(&self, transmission_id: TransmissionID<N>) -> Result<()>;

    /// Removes the transmission for the given `round` and `transmission ID` from storage.
    fn remove_transmission_for_round(&self, round: u64, transmission_id: TransmissionID<N>) -> Result<()>;

    /// Returns `true` if the given `transmission ID` exists.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool>;

    /// Returns `true` if the given `round` and `transmission ID` exists.
    fn contains_transmission_for_round(&self, round: u64, transmission_id: &TransmissionID<N>) -> Result<bool>;

    /// Returns the transmission for the given `transmission ID`.
    fn get_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<Option<Transmission<N>>>;

    /// Returns the transmission for the given `round` and `transmission ID`.
    fn get_transmission_for_round(
        &self,
        round: u64,
        transmission_id: &TransmissionID<N>,
    ) -> Result<Option<Transmission<N>>>;
}
