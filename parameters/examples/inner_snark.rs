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

use snarkos_algorithms::crh::sha256::sha256;
use snarkos_dpc::base_dpc::{
    inner_circuit::InnerCircuit,
    instantiated::Components,
    parameters::SystemParameters,
    BaseDPCComponents,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::{MerkleParameters, SNARK},
    parameters::Parameters,
};
use snarkos_parameters::LedgerMerkleTreeParameters;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::thread_rng;
use std::path::PathBuf;

mod utils;
use utils::store;

pub fn setup<C: BaseDPCComponents>() -> Result<(Vec<u8>, Vec<u8>), DPCError> {
    let rng = &mut thread_rng();

    // TODO (howardwu): Resolve this inconsistency on import structure with a new model once MerkleParameters are refactored.
    let merkle_tree_hash_parameters: <C::MerkleParameters as MerkleParameters>::H =
        From::from(FromBytes::read(&LedgerMerkleTreeParameters::load_bytes()?[..])?);
    let ledger_merkle_tree_parameters = From::from(merkle_tree_hash_parameters);

    let system_parameters = SystemParameters::<C>::load()?;
    let inner_snark_parameters = C::InnerSNARK::setup(
        &InnerCircuit::blank(&system_parameters, &ledger_merkle_tree_parameters),
        rng,
    )?;
    let inner_snark_pk = to_bytes![inner_snark_parameters.0]?;
    let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters = inner_snark_parameters.1.into();
    let inner_snark_vk = to_bytes![inner_snark_vk]?;

    println!("inner_snark_pk.params\n\tsize - {}", inner_snark_pk.len());
    println!("inner_snark_vk.params\n\tsize - {}", inner_snark_vk.len());
    Ok((inner_snark_pk, inner_snark_vk))
}

fn versioned_filename(checksum: &str) -> String {
    match checksum.get(0..7) {
        Some(sum) => format!("inner_snark_pk-{}.params", sum),
        _ => "inner_snark_pk.params".to_string(),
    }
}

pub fn main() {
    let (inner_snark_pk, inner_snark_vk) = setup::<Components>().unwrap();
    let inner_snark_pk_checksum = hex::encode(sha256(&inner_snark_pk));
    store(
        &PathBuf::from(&versioned_filename(&inner_snark_pk_checksum)),
        &PathBuf::from("inner_snark_pk.checksum"),
        &inner_snark_pk,
    )
    .unwrap();
    store(
        &PathBuf::from("inner_snark_vk.params"),
        &PathBuf::from("inner_snark_vk.checksum"),
        &inner_snark_vk,
    )
    .unwrap();
}
