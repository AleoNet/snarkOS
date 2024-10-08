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

use crate::StorageService;
use snarkvm::{
    ledger::{
        narwhal::{BatchHeader, Transmission, TransmissionID},
        store::{
            cow_to_cloned,
            helpers::{
                rocksdb::{
                    internal::{self, BFTMap, Database, MapID},
                    DataMap,
                },
                Map,
                MapRead,
            },
        },
    },
    prelude::{bail, Field, Network, Result},
};

use aleo_std::StorageMode;
use indexmap::{indexset, IndexSet};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};
use tracing::error;

/// A BFT persistent storage service.
#[derive(Debug)]
pub struct BFTPersistentStorage<N: Network> {
    /// The map of `transmission ID` to `(transmission, certificate IDs)` entries.
    transmissions: DataMap<TransmissionID<N>, (Transmission<N>, IndexSet<Field<N>>)>,
    /// The map of `aborted transmission ID` to `certificate IDs` entries.
    aborted_transmission_ids: DataMap<TransmissionID<N>, IndexSet<Field<N>>>,
}

impl<N: Network> BFTPersistentStorage<N> {
    /// Initializes a new BFT persistent storage service.
    pub fn open(storage_mode: StorageMode) -> Result<Self> {
        Ok(Self {
            transmissions: internal::RocksDB::open_map(N::ID, storage_mode.clone(), MapID::BFT(BFTMap::Transmissions))?,
            aborted_transmission_ids: internal::RocksDB::open_map(
                N::ID,
                storage_mode,
                MapID::BFT(BFTMap::AbortedTransmissionIDs),
            )?,
        })
    }

    /// Initializes a new BFT persistent storage service.
    #[cfg(any(test, feature = "test"))]
    pub fn open_testing(temp_dir: std::path::PathBuf, dev: Option<u16>) -> Result<Self> {
        Ok(Self {
            transmissions: internal::RocksDB::open_map_testing(
                temp_dir.clone(),
                dev,
                MapID::BFT(BFTMap::Transmissions),
            )?,
            aborted_transmission_ids: internal::RocksDB::open_map_testing(
                temp_dir,
                dev,
                MapID::BFT(BFTMap::AbortedTransmissionIDs),
            )?,
        })
    }
}

impl<N: Network> StorageService<N> for BFTPersistentStorage<N> {
    /// Returns `true` if the storage contains the specified `transmission ID`.
    fn contains_transmission(&self, transmission_id: TransmissionID<N>) -> bool {
        // Check if the transmission ID exists in storage.
        match self.transmissions.contains_key_confirmed(&transmission_id) {
            Ok(true) => return true,
            Ok(false) => (),
            Err(error) => error!("Failed to check if transmission ID exists in confirmed storage - {error}"),
        }
        // Check if the transmission ID is in aborted storage.
        match self.aborted_transmission_ids.contains_key_confirmed(&transmission_id) {
            Ok(result) => result,
            Err(error) => {
                error!("Failed to check if aborted transmission ID exists in storage - {error}");
                false
            }
        }
    }

    /// Returns the transmission for the given `transmission ID`.
    /// If the transmission ID does not exist in storage, `None` is returned.
    fn get_transmission(&self, transmission_id: TransmissionID<N>) -> Option<Transmission<N>> {
        // Get the transmission.
        match self.transmissions.get_confirmed(&transmission_id) {
            Ok(Some(Cow::Owned((transmission, _)))) => Some(transmission),
            Ok(Some(Cow::Borrowed((transmission, _)))) => Some(transmission.clone()),
            Ok(None) => None,
            Err(error) => {
                error!("Failed to get transmission from storage - {error}");
                None
            }
        }
    }

    /// Returns the missing transmissions in storage from the given transmissions.
    fn find_missing_transmissions(
        &self,
        batch_header: &BatchHeader<N>,
        mut transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
        aborted_transmissions: HashSet<TransmissionID<N>>,
    ) -> Result<HashMap<TransmissionID<N>, Transmission<N>>> {
        // Initialize a list for the missing transmissions from storage.
        let mut missing_transmissions = HashMap::new();
        // Ensure the declared transmission IDs are all present in storage or the given transmissions map.
        for transmission_id in batch_header.transmission_ids() {
            // If the transmission ID does not exist, ensure it was provided by the caller or aborted.
            if !self.contains_transmission(*transmission_id) {
                // Retrieve the transmission.
                match transmissions.remove(transmission_id) {
                    // Append the transmission if it exists.
                    Some(transmission) => {
                        missing_transmissions.insert(*transmission_id, transmission);
                    }
                    // If the transmission does not exist, check if it was aborted.
                    None => {
                        if !aborted_transmissions.contains(transmission_id) {
                            bail!("Failed to provide a transmission");
                        }
                    }
                }
            }
        }
        Ok(missing_transmissions)
    }

    /// Inserts the given certificate ID for each of the transmission IDs, using the missing transmissions map, into storage.
    fn insert_transmissions(
        &self,
        certificate_id: Field<N>,
        transmission_ids: IndexSet<TransmissionID<N>>,
        aborted_transmission_ids: HashSet<TransmissionID<N>>,
        mut missing_transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
    ) {
        // Inserts the following:
        //   - Inserts **only the missing** transmissions from storage.
        //   - Inserts the certificate ID into the corresponding set for **all** transmissions.
        'outer: for transmission_id in transmission_ids {
            // Retrieve the transmission entry.
            match self.transmissions.get_confirmed(&transmission_id) {
                Ok(Some(entry)) => {
                    let (transmission, mut certificate_ids) = cow_to_cloned!(entry);
                    // Insert the certificate ID into the set.
                    certificate_ids.insert(certificate_id);
                    // Update the transmission entry.
                    if let Err(e) = self.transmissions.insert(transmission_id, (transmission, certificate_ids)) {
                        error!("Failed to insert transmission {transmission_id} into storage - {e}");
                        continue 'outer;
                    }
                }
                Ok(None) => {
                    // Retrieve the missing transmission.
                    let Some(transmission) = missing_transmissions.remove(&transmission_id) else {
                        if !aborted_transmission_ids.contains(&transmission_id)
                            && !self.contains_transmission(transmission_id)
                        {
                            error!("Failed to provide a missing transmission {transmission_id}");
                        }
                        continue 'outer;
                    };
                    // Prepare the set of certificate IDs.
                    let certificate_ids = indexset! { certificate_id };
                    // Insert the transmission and a new set with the certificate ID.
                    if let Err(e) = self.transmissions.insert(transmission_id, (transmission, certificate_ids)) {
                        error!("Failed to insert transmission {transmission_id} into storage - {e}");
                        continue 'outer;
                    }
                }
                Err(e) => {
                    error!("Failed to process the 'insert' for transmission {transmission_id} into storage - {e}");
                    continue 'outer;
                }
            }
        }
        // Inserts the aborted transmission IDs.
        for aborted_transmission_id in aborted_transmission_ids {
            // Retrieve the transmission entry.
            match self.aborted_transmission_ids.get_confirmed(&aborted_transmission_id) {
                Ok(Some(entry)) => {
                    let mut certificate_ids = cow_to_cloned!(entry);
                    // Insert the certificate ID into the set.
                    certificate_ids.insert(certificate_id);
                    // Update the transmission entry.
                    if let Err(e) = self.aborted_transmission_ids.insert(aborted_transmission_id, certificate_ids) {
                        error!("Failed to insert aborted transmission ID {aborted_transmission_id} into storage - {e}");
                    }
                }
                Ok(None) => {
                    // Prepare the set of certificate IDs.
                    let certificate_ids = indexset! { certificate_id };
                    // Insert the transmission and a new set with the certificate ID.
                    if let Err(e) = self.aborted_transmission_ids.insert(aborted_transmission_id, certificate_ids) {
                        error!("Failed to insert aborted transmission ID {aborted_transmission_id} into storage - {e}");
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to process the 'insert' for aborted transmission ID {aborted_transmission_id} into storage - {e}"
                    );
                }
            }
        }
    }

    /// Removes the certificate ID for the transmissions from storage.
    ///
    /// If the transmission no longer references any certificate IDs, the entry is removed from storage.
    fn remove_transmissions(&self, certificate_id: &Field<N>, transmission_ids: &IndexSet<TransmissionID<N>>) {
        // If this is the last certificate ID for the transmission ID, remove the transmission.
        for transmission_id in transmission_ids {
            // Retrieve the transmission entry.
            match self.transmissions.get_confirmed(transmission_id) {
                Ok(Some(entry)) => {
                    let (transmission, mut certificate_ids) = cow_to_cloned!(entry);
                    // Insert the certificate ID into the set.
                    certificate_ids.swap_remove(certificate_id);
                    // If there are no more certificate IDs for the transmission ID, remove the transmission.
                    if certificate_ids.is_empty() {
                        // Remove the transmission entry.
                        if let Err(e) = self.transmissions.remove(transmission_id) {
                            error!("Failed to remove transmission {transmission_id} (now empty) from storage - {e}");
                        }
                    }
                    // Otherwise, update the transmission entry.
                    else {
                        // Update the transmission entry.
                        if let Err(e) = self.transmissions.insert(*transmission_id, (transmission, certificate_ids)) {
                            error!(
                                "Failed to remove transmission {transmission_id} for certificate {certificate_id} from storage - {e}"
                            );
                        }
                    }
                }
                Ok(None) => { /* no-op */ }
                Err(e) => {
                    error!("Failed to process the 'remove' for transmission {transmission_id} from storage - {e}");
                }
            }
            // Retrieve the aborted transmission ID entry.
            match self.aborted_transmission_ids.get_confirmed(transmission_id) {
                Ok(Some(entry)) => {
                    let mut certificate_ids = cow_to_cloned!(entry);
                    // Insert the certificate ID into the set.
                    certificate_ids.swap_remove(certificate_id);
                    // If there are no more certificate IDs for the transmission ID, remove the transmission.
                    if certificate_ids.is_empty() {
                        // Remove the transmission entry.
                        if let Err(e) = self.aborted_transmission_ids.remove(transmission_id) {
                            error!(
                                "Failed to remove aborted transmission ID {transmission_id} (now empty) from storage - {e}"
                            );
                        }
                    }
                    // Otherwise, update the transmission entry.
                    else {
                        // Update the transmission entry.
                        if let Err(e) = self.aborted_transmission_ids.insert(*transmission_id, certificate_ids) {
                            error!(
                                "Failed to remove aborted transmission ID {transmission_id} for certificate {certificate_id} from storage - {e}"
                            );
                        }
                    }
                }
                Ok(None) => { /* no-op */ }
                Err(e) => {
                    error!(
                        "Failed to process the 'remove' for aborted transmission ID {transmission_id} from storage - {e}"
                    );
                }
            }
        }
    }

    /// Returns a HashMap over the `(transmission ID, (transmission, certificate IDs))` entries.
    #[cfg(any(test, feature = "test"))]
    fn as_hashmap(&self) -> HashMap<TransmissionID<N>, (Transmission<N>, IndexSet<Field<N>>)> {
        use snarkvm::ledger::store::cow_to_copied;
        self.transmissions.iter_confirmed().map(|(k, v)| (cow_to_copied!(k), cow_to_cloned!(v))).collect()
    }
}
