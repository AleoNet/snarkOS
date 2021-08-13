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

// pub use snarkos_storage::validator::FixMode;
use snarkos_consensus::{DynLedger, MerkleLedger};
use snarkos_storage::{key_value::KeyValueStore, DynStorage, MemDb};
use snarkvm_algorithms::{MerkleParameters, CRH};
use snarkvm_dpc::testnet1::{instantiated::Components, Testnet1Components};

use snarkvm_parameters::{LedgerMerkleTreeParameters, Parameter};
use snarkvm_utilities::FromBytes;
use std::sync::Arc;

// Initialize a test blockchain
pub fn initialize_test_blockchain() -> (DynStorage, DynLedger) {
    let ledger_parameters = {
        type Parameters = <Components as Testnet1Components>::MerkleParameters;
        let parameters: <<Parameters as MerkleParameters>::H as CRH>::Parameters =
            FromBytes::read_le(&LedgerMerkleTreeParameters::load_bytes().unwrap()[..]).unwrap();
        let crh = <Parameters as MerkleParameters>::H::from(parameters);
        Arc::new(Parameters::from(crh))
    };

    let ledger = DynLedger(Box::new(
        MerkleLedger::new(ledger_parameters, &[], &[], &[], &[]).unwrap(),
    ));
    let store = Arc::new(KeyValueStore::new(MemDb::new()));
    (store, ledger)
}
