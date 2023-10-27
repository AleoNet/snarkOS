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
    console::types::Field,
    ledger::narwhal::{Transmission, TransmissionID},
    prelude::{Network, Result},
};

use indexmap::IndexSet;
use std::{collections::HashMap, fmt::Debug};

pub trait StorageService<N: Network>:
    IntoIterator<Item = (TransmissionID<N>, Transmission<N>, IndexSet<Field<N>>)> + Debug + Send + Sync
{
    /// Returns `true` if the storage contains the specified `transmission ID`.
    fn contains_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> bool;

    /// Returns the transmission for the given `transmission ID`.
    /// If the transmission ID does not exist in storage, `None` is returned.
    fn get_transmission(&self, transmission_id: impl Into<TransmissionID<N>>) -> Option<Transmission<N>>;

    /// Given a list of transmission IDs, identify and return the transmissions that are missing from storage.
    fn find_missing_transmissions(
        &self,
        transmission_ids: &IndexSet<TransmissionID<N>>,
        transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<HashMap<TransmissionID<N>, Transmission<N>>>;

    /// Inserts the transmissions from the given list of transmission IDs,
    /// using the provided map of missing transmissions.
    fn insert_transmissions(
        &self,
        round: u64,
        certificate_id: Field<N>,
        transmission_ids: IndexSet<TransmissionID<N>>,
        missing_transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<()>;

    /// Removes the transmissions for the given round and certificate ID, from the given list of transmission IDs from storage.
    fn remove_transmissions(
        &self,
        round: u64,
        certificate_id: Field<N>,
        transmission_ids: &IndexSet<TransmissionID<N>>,
    ) -> Result<()>;
}
