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
    // Various HashMap uses in snarkOS:
    //
    // - HashMap<SocketAddr, Vec<Digest>>
    // - HashMap<Digest, Vec<SocketAddr>>

    let seed: u64 = thread_rng().gen();
    let mut rng = XorShiftRng::seed_from_u64(seed);

    let mut inputs = vec![];
    let sizes = vec![1000, 3000, 5000, 7000];
    for size in sizes {
        inputs.push(generate_hashes(&mut rng, size))
    }

    let mut group = c.benchmark_group("hash_maps_insert");

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

fn hash_sets_insert(c: &mut Criterion) {
    // Various HashSet uses in snarkOS:
    //
    // - HashSet<Digest>
    // - HashSet<Vec<u8>> for tx memos, digests, cms, etc...

    let seed: u64 = thread_rng().gen();
    let mut rng = XorShiftRng::seed_from_u64(seed);

    let mut inputs = vec![];
    let sizes = vec![1000, 3000, 5000, 7000];
    for size in sizes {
        inputs.push(generate_hashes(&mut rng, size))
    }

    let mut group = c.benchmark_group("hash_sets_insert");

    for hashes in inputs {
        let size = hashes.len();

        group.bench_with_input(BenchmarkId::new("HashSet", size), &size, |b, _size| {
            b.iter(|| {
                let mut set: HashSet<[u8; 32]> = HashSet::default();

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

fn generate_hashes(rng: &mut XorShiftRng, count: usize) -> Vec<Digest> {
    (0..count)
        .map(|_| {
            let mut hash = [0u8; 32];
            rng.fill_bytes(&mut hash);

            hash
        })
        .collect()
}

criterion_group!(hash_hasher_bench, hash_maps_insert, hash_sets_insert);
criterion_main!(hash_hasher_bench);
