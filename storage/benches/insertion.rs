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

fn insertion(c: &mut Criterion) {
    let temp_dir1 = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    // Create an empty ledger.
    let ledger1: LedgerState<CurrentNetwork, ReadWrite> =
        LedgerState::open_writer_with_increment::<RocksDB, _>(&temp_dir1, 1).expect("Failed to initialize ledger");
    // Import a dump of a ledger containing 1k blocks.
    ledger1
        .storage()
        .import("benches/storage_1k_blocks")
        .expect("Couldn't import the test ledger");
    // Reopen the ledger so that it applies the storage changes to its in-memory components.
    drop(ledger1);
    let ledger1: LedgerState<CurrentNetwork, ReadWrite> =
        LedgerState::open_writer_with_increment::<RocksDB, _>(&temp_dir1, NUM_BLOCKS).expect("Failed to initialize ledger");

    // Prepare a second test ledger that will be importing blocks belonging to the first one.
    let temp_dir2 = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    let ledger2 = LedgerState::open_writer_with_increment::<RocksDB, _>(temp_dir2, 1).expect("Failed to initialize ledger");

    let mut i = 1;
    c.bench_function("add_block", |b| {
        b.iter(|| {
            let next_block = if let Ok(block) = ledger1.get_block(i) {
                block
            } else {
                let _ = ledger2.revert_to_block_height(0);
                i = 1;
                ledger1.get_block(i).expect("Couldn't find an expected test block")
            };
            ledger2.add_next_block(&next_block).expect("Failed to add a test block");
            i += 1;
        })
    });
}

criterion_group!(
    name = benches;
    // This benchmark needs a bit more time than the default 5s.
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = insertion
);
criterion_main!(benches);
