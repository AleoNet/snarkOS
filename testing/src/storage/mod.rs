// Copyright (C) 2019-2021 Aleo Systems Inc.
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

#[cfg(test)]
pub mod exporter;

#[cfg(test)]
pub mod trim;

#[cfg(test)]
pub mod validator;

pub use snarkos_storage::{Ledger, LedgerStorage};
use snarkvm::{
    dpc::{testnet1::Testnet1Parameters, Parameters, Transaction},
    ledger::{Block, LedgerScheme, Storage},
};

use rand::{thread_rng, Rng};

pub type Store = Ledger<Testnet1Parameters, LedgerStorage>;

pub fn random_storage_path() -> String {
    let random_path: usize = thread_rng().gen();
    format!("./test_db-{}", random_path)
}

// Initialize a test blockchain given genesis attributes
pub fn initialize_test_blockchain<C: Parameters, S: Storage>(genesis_block: Block<Transaction<C>>) -> Ledger<C, S> {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    Ledger::<C, S>::new(Some(&path), genesis_block).unwrap()
}
