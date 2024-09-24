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

use snarkvm::{
    ledger::narwhal::{BatchHeader, Transmission, TransmissionID},
    prelude::{Field, Network, Result},
};

use indexmap::IndexSet;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

pub trait StorageService<N: Network>: Debug + Send + Sync {
    /// Returns `true` if the storage contains the specified `transmission ID`.
    fn contains_transmission(&self, transmission_id: TransmissionID<N>) -> bool;

    /// Returns the transmission for the given `transmission ID`.
    /// If the transmission ID does not exist in storage, `None` is returned.
    fn get_transmission(&self, transmission_id: TransmissionID<N>) -> Option<Transmission<N>>;

    /// Returns the missing transmissions in storage from the given transmissions.
    fn find_missing_transmissions(
        &self,
        batch_header: &BatchHeader<N>,
        transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
        aborted_transmissions: HashSet<TransmissionID<N>>,
    ) -> Result<HashMap<TransmissionID<N>, Transmission<N>>>;

    /// Inserts the given certificate ID for each of the transmission IDs, using the missing transmissions map, into storage.
    fn insert_transmissions(
        &self,
        certificate_id: Field<N>,
        transmission_ids: IndexSet<TransmissionID<N>>,
        aborted_transmission_ids: HashSet<TransmissionID<N>>,
        missing_transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
    );

    /// Removes the certificate ID for the transmissions from storage.
    ///
    /// If the transmission no longer references any certificate IDs, the entry is removed from storage.
    fn remove_transmissions(&self, certificate_id: &Field<N>, transmission_ids: &IndexSet<TransmissionID<N>>);

    /// Returns a HashMap over the `(transmission ID, (transmission, certificate IDs))` entries.
    #[cfg(any(test, feature = "test"))]
    fn as_hashmap(&self) -> HashMap<TransmissionID<N>, (Transmission<N>, IndexSet<Field<N>>)>;
}
