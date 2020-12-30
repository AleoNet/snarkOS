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

use snarkos_errors::algorithms::CRHError;
use snarkos_models::{algorithms::CRH, dpc::DPCComponents};
use snarkos_utilities::{bytes::ToBytes, to_bytes};
use snarkvm_dpc::base_dpc::instantiated::Components;

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let serial_number_nonce_crh = <C::SerialNumberNonceCRH as CRH>::setup(rng);
    let serial_number_nonce_crh_parameters = serial_number_nonce_crh.parameters();
    let serial_number_nonce_crh_parameters_bytes = to_bytes![serial_number_nonce_crh_parameters]?;

    let size = serial_number_nonce_crh_parameters_bytes.len();
    println!("serial_number_nonce_crh.params\n\tsize - {}", size);
    Ok(serial_number_nonce_crh_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("serial_number_nonce_crh.params");
    let sumname = PathBuf::from("serial_number_nonce_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
