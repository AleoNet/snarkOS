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

use snarkos_errors::dpc::DPCError;
use snarkos_models::algorithms::SNARK;
use snarkos_utilities::{bytes::ToBytes, to_bytes};
use snarkvm_dpc::base_dpc::{instantiated::Components, parameters::SystemParameters, BaseDPCComponents, DPC};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: BaseDPCComponents>() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();
    let system_parameters = SystemParameters::<C>::load()?;

    let noop_program_snark_parameters = DPC::<C>::generate_noop_program_snark_parameters(&system_parameters, rng)?;
    let noop_program_snark_pk = to_bytes![noop_program_snark_parameters.proving_key]?;
    let noop_program_snark_vk: <C::NoopProgramSNARK as SNARK>::VerificationParameters =
        noop_program_snark_parameters.verification_key;
    let noop_program_snark_vk = to_bytes![noop_program_snark_vk]?;

    println!("noop_program_snark_pk.params\n\tsize - {}", noop_program_snark_pk.len());
    println!("noop_program_snark_vk.params\n\tsize - {}", noop_program_snark_vk.len());
    Ok((noop_program_snark_pk, noop_program_snark_vk))
}

pub fn main() {
    let (program_snark_pk, program_snark_vk) = setup::<Components>().unwrap();
    store(
        &PathBuf::from("noop_program_snark_pk.params"),
        &PathBuf::from("noop_program_snark_pk.checksum"),
        &program_snark_pk,
    )
    .unwrap();
    store(
        &PathBuf::from("noop_program_snark_vk.params"),
        &PathBuf::from("noop_program_snark_vk.checksum"),
        &program_snark_vk,
    )
    .unwrap();
}
