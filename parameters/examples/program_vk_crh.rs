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

use snarkvm_dpc::base_dpc::instantiated::Components;
use snarkvm_errors::algorithms::CRHError;
use snarkvm_models::{algorithms::CRH, dpc::DPCComponents};
use snarkvm_utilities::{bytes::ToBytes, to_bytes};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: DPCComponents>() -> Result<Vec<u8>, CRHError> {
    let rng = &mut thread_rng();
    let program_vk_crh = <C::ProgramVerificationKeyCRH as CRH>::setup(rng);
    let program_vk_crh_parameters = program_vk_crh.parameters();
    let program_vk_crh_parameters_bytes = to_bytes![program_vk_crh_parameters]?;

    let size = program_vk_crh_parameters_bytes.len();
    println!("program_vk_crh.params\n\tsize - {}", size);
    Ok(program_vk_crh_parameters_bytes)
}

pub fn main() {
    let bytes = setup::<Components>().unwrap();
    let filename = PathBuf::from("program_vk_crh.params");
    let sumname = PathBuf::from("program_vk_crh.checksum");
    store(&filename, &sumname, &bytes).unwrap();
}
