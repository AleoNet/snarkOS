// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use snarkos_environment::CurrentNetwork;
use snarkos_storage::{
    storage::{rocksdb::RocksDB, ReadWrite, Storage},
    LedgerState,
};

use criterion::{criterion_group, criterion_main, Criterion};

use std::time::Duration;

// This value should be no greater than the number of blocks available in the loaded dump.
const NUM_BLOCKS: u32 = 1_000;

fn opening(c: &mut Criterion) {
    let temp_dir = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    // Create an empty ledger.
    let ledger: LedgerState<CurrentNetwork, ReadWrite> =
        LedgerState::open_writer_with_increment::<RocksDB, _>(&temp_dir, 1).expect("Failed to initialize ledger");
    // Import a dump of a ledger containing 1k blocks.
    ledger
        .storage()
        .import("benches/storage_1k_blocks")
        .expect("Couldn't import the test ledger");
    // Drop the ledger, as we will be benchmarking re-opening it.
    drop(ledger);

    c.bench_function("Ledger::open_writer", |b| {
        b.iter(|| {
            let _ledger: LedgerState<CurrentNetwork, ReadWrite> =
                LedgerState::open_writer_with_increment::<RocksDB, _>(&temp_dir, NUM_BLOCKS).expect("Failed to initialize ledger");
        })
    });
}

criterion_group!(
    name = benches;
    // This benchmark needs quite a bit more time than the default 5s.
    config = Criterion::default().measurement_time(Duration::from_secs(60));
    targets = opening
);
criterion_main!(benches);
