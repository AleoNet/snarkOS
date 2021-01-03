// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use snarkvm_dpc::base_dpc::instantiated::Components;
use snarkvm_errors::algorithms::CommitmentError;
use snarkvm_models::{algorithms::CommitmentScheme, dpc::DPCComponents};
use snarkvm_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CommitmentError> {
    let rng = &mut thread_rng();
    let local_data_commitment = <C::LocalDataCommitment as CommitmentScheme>::setup(rng);
    let local_data_commitment_parameters = local_data_commitment.parameters();
    let local_data_commitment_parameters_bytes = to_bytes![local_data_commitment_parameters]?;

    let size = local_data_commitment_parameters_bytes.len();
    println!("local_data_commitment.params\n\tsize - {}", size);
    Ok(local_data_commitment_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("local_data_commitment.params");
    let sumname = PathBuf::from("local_data_commitment.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
