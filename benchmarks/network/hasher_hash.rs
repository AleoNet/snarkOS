// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use criterion::*;
use hash_hasher::{HashBuildHasher, HashedMap, HashedSet};
use rand::{thread_rng, Rng, RngCore, SeedableRng};
use rand_xorshift::XorShiftRng;

use std::collections::{HashMap, HashSet};

// The current applicable digest size for blocks.
//
// Note: `snarkos_storage::Digest` isn't implemented as a fixed size.
type Digest = [u8; 32];

fn hash_maps_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_maps_insert");

    // Generate the hashes to be used in the benchmarks.
    let inputs = generate_inputs();

    for hashes in inputs {
        let size = hashes.len();

        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |b, _size| {
            b.iter(|| {
                let mut map: HashMap<Digest, usize> = HashMap::default();

                for (i, hash) in hashes.iter().enumerate() {
                    map.insert(*hash, i);
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("HashedMap", size), &size, |b, _size| {
            b.iter(|| {
                let mut map = HashedMap::with_hasher(HashBuildHasher::default());

                for (i, hash) in hashes.iter().enumerate() {
                    map.insert(*hash, i);
                }
            })
        });
    }

    group.finish();
}

fn hash_maps_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_maps_get");

    // Generate the hashes to be used in the benchmarks.
    let inputs = generate_inputs();

    for hashes in inputs {
        let size = hashes.len();

        let hash_map: HashMap<Digest, usize> = hashes.iter().enumerate().map(|(i, &hash)| (hash, i)).collect();
        let hashed_map: HashedMap<Digest, usize> = hashes.iter().enumerate().map(|(i, &hash)| (hash, i)).collect();

        group.bench_with_input(BenchmarkId::new("HashMap", size), &size, |b, _size| {
            b.iter(|| {
                for hash in hashes.iter() {
                    hash_map.get(hash);
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("HashedMap", size), &size, |b, _size| {
            b.iter(|| {
                for hash in hashes.iter() {
                    hashed_map.get(hash);
                }
            })
        });
    }

    group.finish();
}

fn hash_sets_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_sets_insert");

    // Generate the hashes to be used in the benchmarks.
    let inputs = generate_inputs();

    for hashes in inputs {
        let size = hashes.len();

        group.bench_with_input(BenchmarkId::new("HashSet", size), &size, |b, _size| {
            b.iter(|| {
                let mut set: HashSet<Digest> = HashSet::default();

                for hash in &hashes {
                    set.insert(*hash);
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("HashedSet", size), &size, |b, _size| {
            b.iter(|| {
                let mut set = HashedSet::with_hasher(HashBuildHasher::default());

                for hash in &hashes {
                    set.insert(*hash);
                }
            })
        });
    }

    group.finish();
}

fn hash_sets_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_sets_contains");

    // Generate the hashes to be used in the benchmarks.
    let inputs = generate_inputs();

    for hashes in inputs {
        let size = hashes.len();

        let hash_set: HashSet<Digest> = hashes.iter().copied().collect();
        let hashed_set: HashedSet<Digest> = hashes.iter().copied().collect();

        group.bench_with_input(BenchmarkId::new("HashSet", size), &size, |b, _size| {
            b.iter(|| {
                for hash in &hashes {
                    hash_set.contains(hash);
                }
            })
        });

        group.bench_with_input(BenchmarkId::new("HashedSet", size), &size, |b, _size| {
            b.iter(|| {
                for hash in &hashes {
                    hashed_set.contains(hash);
                }
            })
        });
    }

    group.finish();
}

fn generate_inputs() -> Vec<Vec<Digest>> {
    let seed: u64 = thread_rng().gen();
    let mut rng = XorShiftRng::seed_from_u64(seed);

    let mut inputs = vec![];
    let sizes = vec![1000, 3000, 5000, 7000];
    for size in sizes {
        inputs.push(generate_hashes(&mut rng, size))
    }

    inputs
}

fn generate_hashes(rng: &mut XorShiftRng, count: usize) -> Vec<Digest> {
    (0..count)
        .map(|_| {
            let mut hash = [0u8; 32];
            rng.fill_bytes(&mut hash);

            hash
        })
        .collect()
}

criterion_group!(
    hash_hasher_bench,
    hash_maps_insert,
    hash_maps_get,
    hash_sets_insert,
    hash_sets_contains
);
criterion_main!(hash_hasher_bench);
