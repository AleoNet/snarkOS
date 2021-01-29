// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use snarkvm_algorithms::crh::sha256::sha256;
use snarkvm_dpc::base_dpc::instantiated::Components;
use snarkvm_errors::algorithms::CRHError;
use snarkvm_models::{algorithms::CRH, dpc::DPCComponents};
use snarkvm_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::{
    fs::{self, File},
    io::{Result as IoResult, Write},
    path::PathBuf,
};

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let encrypted_record_crh = <C::EncryptedRecordCRH as CRH>::setup(rng);
    let encrypted_record_crh_parameters = encrypted_record_crh.parameters();
    let encrypted_record_crh_parameters_bytes = to_bytes![encrypted_record_crh_parameters]?;

    let size = encrypted_record_crh_parameters_bytes.len();
    println!("encrypted_record_crh.params\n\tsize - {}", size);
    Ok(encrypted_record_crh_parameters_bytes)
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
    let filename = PathBuf::from("encrypted_record_crh.params");
    let sumname = PathBuf::from("encrypted_record_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
