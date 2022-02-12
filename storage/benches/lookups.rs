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
use snarkvm::{dpc::testnet2::Testnet2, prelude::Block, traits::Network};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{prelude::SliceRandom, thread_rng, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

use std::fs;

const NUM_BLOCKS: usize = 1_000;

fn lookups(c: &mut Criterion) {
    // Prepare the test ledger.
    let temp_dir = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    let ledger = LedgerState::open_writer_with_increment::<RocksDB, _>(temp_dir, 1).expect("Failed to initialize ledger");

    // Read the test blocks; note: they don't include the genesis block, as it's always available when creating a ledger.
    // note: the `blocks_100` and `blocks_1000` files were generated on a testnet2 storage using `LedgerState::dump_blocks`.
    let test_blocks = fs::read(format!("benches/blocks_{}", NUM_BLOCKS)).unwrap_or_else(|_| panic!("Missing the test blocks file"));
    let blocks: Vec<Block<Testnet2>> = bincode::deserialize(&test_blocks).expect("Failed to deserialize a block dump");
    assert_eq!(blocks.len(), NUM_BLOCKS - 1);

    // Prepare the collections for block component ids.

    let mut block_hashes = Vec::with_capacity(NUM_BLOCKS);
    block_hashes.push(Testnet2::genesis_block().hash());

    let mut ledger_roots = Vec::with_capacity(NUM_BLOCKS);

    let mut tx_ids = Vec::with_capacity(NUM_BLOCKS);
    for tx_id in Testnet2::genesis_block().transactions().transaction_ids() {
        tx_ids.push(tx_id);
    }

    let mut tx_commitments = Vec::with_capacity(NUM_BLOCKS);
    for tx_commitment in Testnet2::genesis_block().transactions().commitments() {
        tx_commitments.push(tx_commitment);
    }

    let mut tx_serial_numbers = Vec::with_capacity(NUM_BLOCKS);
    for tx_serial_number in Testnet2::genesis_block().transactions().serial_numbers() {
        tx_serial_numbers.push(tx_serial_number);
    }

    // Load the test blocks into the ledger and register their components along the way.
    for block in &blocks {
        ledger.add_next_block(block).expect("Failed to add a test block");
        block_hashes.push(block.hash());
        ledger_roots.push(ledger.latest_ledger_root());
        tx_ids.extend(block.transactions().transaction_ids());
        tx_commitments.extend(block.commitments());
        tx_serial_numbers.extend(block.serial_numbers());
    }
    assert_eq!(block_hashes.len(), NUM_BLOCKS);

    // Seed a fast random number generator.
    let seed: u64 = thread_rng().gen();
    let mut rng = XorShiftRng::seed_from_u64(seed);

    c.bench_function("ledger_roots_lookup", |b| {
        b.iter(|| {
            let root = ledger_roots.choose(&mut rng).unwrap();
            ledger.contains_ledger_root(root).expect("Lookup by ledger root failed");
        })
    });

    c.bench_function("blocks_lookup_by_height", |b| {
        b.iter(|| {
            let height = rng.gen_range(0..NUM_BLOCKS as u32);
            ledger.contains_block_height(height).expect("Lookup by block height failed");
        })
    });

    c.bench_function("blocks_lookup_by_hash", |b| {
        b.iter(|| {
            let hash = block_hashes.choose(&mut rng).unwrap();
            ledger.contains_block_hash(hash).expect("Lookup by block hash failed");
        })
    });

    c.bench_function("txs_lookup_by_id", |b| {
        b.iter(|| {
            let id = tx_ids.choose(&mut rng).unwrap();
            ledger.contains_transaction(id).expect("Lookup by tx id failed");
        })
    });

    c.bench_function("txs_lookup_by_commitment", |b| {
        b.iter(|| {
            let id = tx_commitments.choose(&mut rng).unwrap();
            ledger.contains_commitment(id).expect("Lookup by commitment failed");
        })
    });

    c.bench_function("txs_lookup_by_serial_number", |b| {
        b.iter(|| {
            let id = tx_serial_numbers.choose(&mut rng).unwrap();
            ledger.contains_serial_number(id).expect("Lookup by serial number failed");
        })
    });
}

criterion_group!(benches, lookups);
criterion_main!(benches);
