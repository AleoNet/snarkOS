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

use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_errors::algorithms::CRHError;
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};
use snarkvm_algorithms::crh::sha256::sha256;

use rand::thread_rng;
use std::{
    fs::{self, File},
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let local_data_crh = <C::LocalDataCRH as CRH>::setup(rng);
    let local_data_crh_parameters = local_data_crh.parameters();
    let local_data_crh_parameters_bytes = to_bytes![local_data_crh_parameters]?;

    let size = local_data_crh_parameters_bytes.len();
    println!("local_data_crh.params\n\tsize - {}", size);
    Ok(local_data_crh_parameters_bytes)
}

pub fn store(file_path: &PathBuf, checksum_path: &PathBuf, bytes: &[u8]) -> IoResult<()> {
    // Save checksum to file
    fs::write(checksum_path, hex::encode(sha256(bytes)))?;

    // Save buffer to file
    let mut file = File::create(file_path)?;
    file.write_all(&bytes)?;
    drop(file);
    Ok(())
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("local_data_crh.params");
    let sumname = PathBuf::from("local_data_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
