// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use snarkos_toolkit::account::{Address, PrivateKey};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

fn account_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("account");
    group.sample_size(20);

    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);

    group.bench_function("private_key", |b| {
        b.iter(|| {
            let _private_key = PrivateKey::new(rng).unwrap();
        });
    });

    let private_key = PrivateKey::new(rng).unwrap();

    group.bench_function("address", |b| {
        b.iter(|| {
            let _address = Address::from(&private_key).unwrap();
        });
    });
}

criterion_group!(benches, account_bench);

criterion_main!(benches);
