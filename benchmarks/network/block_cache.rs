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
use rand::{thread_rng, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

use snarkos_network::Cache;

fn block_cache_perf(c: &mut Criterion) {
    const MIN_BLOCK_SIZE: usize = 1024; // 1KiB
    const MAX_BLOCK_SIZE: usize = 1024 * 1024; // 1MiB

    let mut cache = Cache::default();

    let seed: u64 = thread_rng().gen();
    let mut rng = XorShiftRng::seed_from_u64(seed);

    c.bench_function("cache_bench", move |b| {
        b.iter(|| {
            let block_size: usize = rng.gen_range(MIN_BLOCK_SIZE..MAX_BLOCK_SIZE);

            let mut random_bytes = vec![0u8; block_size];
            rng.fill(&mut random_bytes[..]);

            let _check = cache.contains(&random_bytes);
        })
    });
}

criterion_group!(cache_bench, block_cache_perf);

criterion_main!(cache_bench);
