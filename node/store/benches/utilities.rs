// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use snarkvm::prelude::*;

use anyhow::Result;
use snarkos_node_store::ProgramDB;
use std::str::FromStr;

fn sample_key_value_pairs(
    num_key_value_pairs: usize,
    rng: &mut TestRng,
) -> Vec<(Plaintext<Testnet3>, Value<Testnet3>)> {
    let mut key_value_pairs = Vec::with_capacity(num_key_value_pairs);

    let value = Value::<Testnet3>::from_str(&format!("{}", Group::<Testnet3>::rand(rng))).unwrap();

    for i in 0..num_key_value_pairs {
        let key = Plaintext::<Testnet3>::from_str(&format!("{i}u32")).unwrap();
        key_value_pairs.push((key, value.clone()));
    }

    key_value_pairs
}

pub fn populate_program_memory(
    program_store: &mut ProgramDB<Testnet3>,
    parameters: &[(&str, &str, usize)],
    rng: &mut TestRng,
) -> Result<()> {
    // For each program and mapping pair, add the desired number of random key-value pairs.
    for (program_name, mapping_name, num_entries) in parameters.iter() {
        // Construct the program ID.
        let program_id = ProgramID::<Testnet3>::from_str(*program_name).unwrap();

        // Construct the mapping name.
        let mapping_name = Identifier::from_str(*mapping_name)?;

        // Initialize the mapping if it does not exist.
        if !program_store.contains_mapping(&program_id, &mapping_name).unwrap() {
            program_store.initialize_mapping(&program_id, &mapping_name)?
        }

        // Sample the key-value pairs.
        let key_value_pairs = sample_key_value_pairs(*num_entries, rng);

        // Insert the key-value pairs.
        for (key, value) in key_value_pairs.iter() {
            program_store.update_key_value(&program_id, &mapping_name, key.clone(), value.clone())?;
        }
    }
    Ok(())
}
