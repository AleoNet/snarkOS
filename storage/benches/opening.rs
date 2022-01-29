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

use snarkos_storage::{storage::rocksdb::RocksDB, LedgerState};
use snarkvm::{dpc::testnet2::Testnet2, prelude::Block};

use criterion::{criterion_group, criterion_main, Criterion};

use std::{fs, time::Duration};

const NUM_BLOCKS: usize = 1_000;

fn opening(c: &mut Criterion) {
    // Read the test blocks.
    // note: the `blocks_100` and `blocks_1000` files were generated on a testnet2 storage using `LedgerState::dump_blocks`.
    let mut test_blocks = fs::read(format!("benches/blocks_{}", NUM_BLOCKS)).expect(&format!("Missing the test blocks file"));
    let blocks: Vec<Block<Testnet2>> = bincode::deserialize(&mut test_blocks).expect("Failed to deserialize a block dump");
    assert_eq!(blocks.len(), NUM_BLOCKS - 1);

    // Prepare a test ledger and insert all the test blocks.
    let temp_dir = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    {
        let ledger = LedgerState::open_writer::<RocksDB, _>(&temp_dir).expect("Failed to initialize ledger");
        for block in &blocks {
            ledger.add_next_block(block).expect("Failed to add a test block");
        }
    }

    c.bench_function("Ledger::open_writer", |b| {
        b.iter(|| {
            let _ledger: LedgerState<Testnet2> = LedgerState::open_writer::<RocksDB, _>(&temp_dir).expect("Failed to initialize ledger");
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
