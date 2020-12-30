// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::consensus::TestTx;
pub use snarkos_storage::Ledger;
use snarkvm_dpc::base_dpc::instantiated::CommitmentMerkleParameters;
use snarkvm_models::{
    algorithms::merkle_tree::LoadableMerkleParameters,
    objects::{LedgerScheme, Transaction},
};
use snarkvm_objects::Block;

use rand::{thread_rng, Rng};
use std::{path::PathBuf, sync::Arc};

pub type Store = Ledger<TestTx, CommitmentMerkleParameters>;

pub fn random_storage_path() -> String {
    let random_path: usize = thread_rng().gen();
    format!("./test_db-{}", random_path)
}

// Initialize a test blockchain given genesis attributes
pub fn initialize_test_blockchain<T: Transaction, P: LoadableMerkleParameters>(
    parameters: P,
    genesis_block: Block<T>,
) -> Ledger<T, P> {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    Ledger::<T, P>::destroy_storage(path.clone()).unwrap();

    Ledger::<T, P>::new(&path, parameters, genesis_block).unwrap()
}

// Open a test blockchain from stored genesis attributes
pub fn open_test_blockchain<T: Transaction, P: LoadableMerkleParameters>() -> (Arc<Ledger<T, P>>, PathBuf) {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    Ledger::<T, P>::destroy_storage(path.clone()).unwrap();

    let storage = Arc::new(Ledger::<T, P>::open_at_path(path.clone()).unwrap());

    (storage, path)
}

pub fn kill_storage<T: Transaction, P: LoadableMerkleParameters>(ledger: Ledger<T, P>) {
    let path = ledger.storage.db.path().to_owned();

    drop(ledger);
    Ledger::<T, P>::destroy_storage(path).unwrap();
}

pub fn kill_storage_async<T: Transaction, P: LoadableMerkleParameters>(path: PathBuf) {
    Ledger::<T, P>::destroy_storage(path).unwrap();
}

pub fn kill_storage_sync<T: Transaction, P: LoadableMerkleParameters>(ledger: Arc<Ledger<T, P>>) {
    let path = ledger.storage.db.path().to_owned();

    drop(ledger);
    Ledger::<T, P>::destroy_storage(path).unwrap();
}
