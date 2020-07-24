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
