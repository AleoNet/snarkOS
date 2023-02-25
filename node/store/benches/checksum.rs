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

#[macro_use]
extern crate criterion;

#[path = "./utilities.rs"]
mod utilities;
use utilities::*;

use snarkvm::prelude::*;

use anyhow::Result;
use std::iter;

use criterion::Criterion;
use snarkos_node_store::ProgramDB;
use snarkvm::prelude::helpers::MapRead;

// A helper function to lazily prepare the input for the checksum computation.
// This function avoids constructing the entire input in memory, which is useful in benchmarking the raw performance of the checksum.
fn prepare_bit_streams(parameters: &[usize]) -> impl Iterator<Item = impl Iterator<Item = bool>> + '_ {
    parameters.iter().map(move |mapping_size_in_bits| {
        let mapping_id = Field::<Testnet3>::zero();
        mapping_id.to_bits_le().into_iter().chain(iter::repeat(false).take(*mapping_size_in_bits))
    })
}

// A helper function to run the benchmark, parameterized by the outer and inner hash functions.
fn run_checksum_benchmark(
    parameters: &[usize],
    outer: fn(&[bool]) -> Result<Field<Testnet3>>,
    inner: fn(&[bool]) -> Result<Field<Testnet3>>,
) {
    let input = prepare_bit_streams(parameters);
    let mapping_checksums = input.map(|bits| {
        // Compute the mapping checksum as `Hash( bits )`.
        let mapping_checksum = inner(&bits.collect::<Vec<_>>()).expect("Could not compute mapping checksum");
        // Return the bits of the mapping checksum.
        mapping_checksum.to_bits_le()
    });
    // Compute the storage root as `Hash( mapping_checksums )`.
    outer(&mapping_checksums.flatten().collect::<Vec<_>>()).expect("Could not compute storage root");
}

fn compare_checksum_and_storage_root(c: &mut Criterion) {
    let mut group = c.benchmark_group("Compare Checksum and Storage Root");

    // Open the program DB.
    let mut program_store = ProgramDB::<Testnet3>::open(Some(0)).unwrap();

    // Initialize the rng.
    let rng = &mut TestRng::default();

    for i in 10..20 {
        // Populate the program store.
        populate_program_memory(&mut program_store, &[("foo.aleo", "bar", 1 << i)], rng).unwrap();

        // Calculate the size of the program store in bits.
        let program_store_size_in_bits = program_store
            .key_value_id_map()
            .iter()
            .map(|(mapping_id, key_value_ids)| {
                mapping_id.to_bits_le().len()
                    + key_value_ids.values().map(|value_id| value_id.to_bits_le().len()).sum::<usize>()
            })
            .sum();

        // Benchmark the storage root computation.
        group.bench_function(&format!("Storage root: 1 mapping w/ {} key-value pairs", 1 << i), |b| {
            b.iter(|| program_store.get_checksum().unwrap())
        });

        // Benchmark the checksum computation.
        group.bench_with_input(
            format!("Checksum: {} bits", program_store_size_in_bits),
            &[program_store_size_in_bits],
            |b, parameters| {
                b.iter(|| run_checksum_benchmark(parameters, Testnet3::hash_bhp1024, Testnet3::hash_bhp1024))
            },
        );
    }

    group.finish()
}

// Benchmark the checksum computation over a stream of bits.
criterion_group! {
    name = checksum;
    config = Criterion::default().sample_size(10);
    targets = compare_checksum_and_storage_root
}

criterion_main!(checksum);
