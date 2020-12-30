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

use snarkos_dpc::base_dpc::{
    inner_circuit::InnerCircuit,
    instantiated::Components,
    outer_circuit::OuterCircuit,
    parameters::{NoopProgramSNARKParameters, SystemParameters},
    program::{NoopCircuit, PrivateProgramInput},
    BaseDPCComponents,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{MerkleParameters, SNARK},
    parameters::Parameters,
};
use snarkos_parameters::{InnerSNARKPKParameters, InnerSNARKVKParameters, LedgerMerkleTreeParameters};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};
use snarkvm_algorithms::crh::sha256::sha256;

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: BaseDPCComponents>() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();
    let system_parameters = SystemParameters::<C>::load()?;

    let merkle_tree_hash_parameters: <C::MerkleParameters as MerkleParameters>::H =
        From::from(FromBytes::read(&LedgerMerkleTreeParameters::load_bytes()?[..])?);
    let ledger_merkle_tree_parameters = From::from(merkle_tree_hash_parameters);

    let inner_snark_pk: <C::InnerSNARK as SNARK>::ProvingParameters =
        <C::InnerSNARK as SNARK>::ProvingParameters::read(InnerSNARKPKParameters::load_bytes()?.as_slice())?;

    let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
        <C::InnerSNARK as SNARK>::VerificationParameters::read(InnerSNARKVKParameters::load_bytes()?.as_slice())?;

    let inner_snark_proof = C::InnerSNARK::prove(
        &inner_snark_pk,
        &InnerCircuit::blank(&system_parameters, &ledger_merkle_tree_parameters),
        rng,
    )?;

    // TODO (howardwu): Check why is the PrivateProgramInput necessary for running the setup? Blank should take option?
    let noop_program_snark_parameters = NoopProgramSNARKParameters::<C>::load()?;

    let program_snark_proof = C::NoopProgramSNARK::prove(
        &noop_program_snark_parameters.proving_key,
        &NoopCircuit::blank(&system_parameters),
        rng,
    )?;
    let private_program_input = PrivateProgramInput {
        verification_key: to_bytes![noop_program_snark_parameters.verification_key]?,
        proof: to_bytes![program_snark_proof]?,
    };

    let outer_snark_parameters = C::OuterSNARK::setup(
        &OuterCircuit::blank(
            system_parameters,
            ledger_merkle_tree_parameters,
            inner_snark_vk,
            inner_snark_proof,
            private_program_input,
        ),
        rng,
    )?;

    let outer_snark_pk = to_bytes![outer_snark_parameters.0]?;
    let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters = outer_snark_parameters.1.into();
    let outer_snark_vk = to_bytes![outer_snark_vk]?;

    println!("outer_snark_pk.params\n\tsize - {}", outer_snark_pk.len());
    println!("outer_snark_vk.params\n\tsize - {}", outer_snark_vk.len());
    Ok((outer_snark_pk, outer_snark_vk))
}

fn versioned_filename(checksum: &str) -> String {
    match checksum.get(0..7) {
        Some(sum) => format!("outer_snark_pk-{}.params", sum),
        _ => "outer_snark_pk.params".to_string(),
    }
}

pub fn main() {
    let (outer_snark_pk, outer_snark_vk) = setup::<Components>().unwrap();
    let outer_snark_pk_checksum = hex::encode(sha256(&outer_snark_pk));
    store(
        &PathBuf::from(&versioned_filename(&outer_snark_pk_checksum)),
        &PathBuf::from("outer_snark_pk.checksum"),
        &outer_snark_pk,
    )
    .unwrap();
    store(
        &PathBuf::from("outer_snark_vk.params"),
        &PathBuf::from("outer_snark_vk.checksum"),
        &outer_snark_vk,
    )
    .unwrap();
}
