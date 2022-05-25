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
    storage::{rocksdb::RocksDB, Map, MapId, Storage},
    LedgerState,
    Metadata,
};
use snarkvm::{prelude::BlockHeader, traits::Network};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::{prelude::SliceRandom, thread_rng, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

// This value should be no greater than the number of blocks available in the loaded dump.
const NUM_BLOCKS: u32 = 1_000;

fn lookups(c: &mut Criterion) {
    let temp_dir = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    // Create an empty ledger.
    let ledger: LedgerState<CurrentNetwork> =
        LedgerState::open_writer_with_increment::<RocksDB, _>(&temp_dir, 1).expect("Failed to initialize ledger");
    // Import a dump of a ledger containing 1k blocks.
    ledger
        .storage()
        .import("benches/storage_1k_blocks")
        .expect("Couldn't import the test ledger");

    // Seed a fast random number generator.
    let seed: u64 = thread_rng().gen();
    let mut rng = XorShiftRng::seed_from_u64(seed);

    c.bench_function("ledger_roots_lookup", |b| {
        let ledger_roots = ledger
            .storage()
            .open_map::<<CurrentNetwork as Network>::LedgerRoot, u32>(MapId::LedgerRoots)
            .unwrap()
            .keys()
            .collect::<Vec<_>>();

        b.iter(|| {
            let root = ledger_roots.choose(&mut rng).unwrap();
            ledger.contains_ledger_root(root).expect("Lookup by ledger root failed");
        })
    });

    c.bench_function("blocks_lookup_by_height", |b| {
        b.iter(|| {
            let height = rng.gen_range(0..NUM_BLOCKS);
            ledger.contains_block_height(height).expect("Lookup by block height failed");
        })
    });

    c.bench_function("blocks_lookup_by_hash", |b| {
        let block_hashes = ledger
            .storage()
            .open_map::<<CurrentNetwork as Network>::BlockHash, BlockHeader<CurrentNetwork>>(MapId::BlockHeaders)
            .unwrap()
            .keys()
            .collect::<Vec<_>>();

        b.iter(|| {
            let hash = block_hashes.choose(&mut rng).unwrap();
            ledger.contains_block_hash(hash).expect("Lookup by block hash failed");
        })
    });

    c.bench_function("txs_lookup_by_id", |b| {
        let tx_ids = ledger
            .storage()
            .open_map::<<CurrentNetwork as Network>::TransactionID, (
                <CurrentNetwork as Network>::LedgerRoot,
                Vec<<CurrentNetwork as Network>::TransitionID>,
                Metadata<CurrentNetwork>,
            )>(MapId::Transactions)
            .unwrap()
            .keys()
            .collect::<Vec<_>>();

        b.iter(|| {
            let id = tx_ids.choose(&mut rng).unwrap();
            ledger.contains_transaction(id).expect("Lookup by tx id failed");
        })
    });

    // Commitments are used for multiple lookups.
    let tx_commitments = ledger
        .storage()
        .open_map::<<CurrentNetwork as Network>::Commitment, <CurrentNetwork as Network>::TransitionID>(MapId::Commitments)
        .unwrap()
        .keys()
        .collect::<Vec<_>>();

    c.bench_function("txs_lookup_by_commitment", |b| {
        b.iter(|| {
            let id = tx_commitments.choose(&mut rng).unwrap();
            ledger.contains_commitment(id).expect("Lookup by commitment failed");
        })
    });

    c.bench_function("ciphertext_lookup_by_commitment", |b| {
        b.iter(|| {
            let id = tx_commitments.choose(&mut rng).unwrap();
            ledger.get_ciphertext(id).expect("Lookup by commitment failed");
        })
    });

    c.bench_function("txs_lookup_by_serial_number", |b| {
        let tx_serial_numbers = ledger
            .storage()
            .open_map::<<CurrentNetwork as Network>::SerialNumber, <CurrentNetwork as Network>::TransitionID>(MapId::SerialNumbers)
            .unwrap()
            .keys()
            .collect::<Vec<_>>();

        b.iter(|| {
            let id = tx_serial_numbers.choose(&mut rng).unwrap();
            ledger.contains_serial_number(id).expect("Lookup by serial number failed");
        })
    });
}

criterion_group!(benches, lookups);
criterion_main!(benches);
