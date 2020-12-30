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

use snarkvm_dpc::base_dpc::{instantiated::Components, BaseDPCComponents};
use snarkvm_errors::algorithms::MerkleError;
use snarkvm_models::algorithms::MerkleParameters;
use snarkvm_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: BaseDPCComponents>() -> Result<Vec<u8>, MerkleError> {
    let rng = &mut thread_rng();

    let ledger_merkle_tree_parameters = <C::MerkleParameters as MerkleParameters>::setup(rng);
    let ledger_merkle_tree_parameters_bytes = to_bytes![ledger_merkle_tree_parameters.parameters()]?;

    let size = ledger_merkle_tree_parameters_bytes.len();
    println!("ledger_merkle_tree.params\n\tsize - {}", size);
    Ok(ledger_merkle_tree_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("ledger_merkle_tree.params");
    let sumname = PathBuf::from("ledger_merkle_tree.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
