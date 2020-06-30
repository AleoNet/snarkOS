use snarkos_toolkit::account::{PrivateKey, PublicKey};

use criterion::{criterion_group, criterion_main, Criterion};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;

fn account_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Account");
    group.sample_size(10);

    let rng = &mut ChaChaRng::seed_from_u64(1231275789u64);

    group.bench_function("private_key", |b| {
        b.iter(|| {
            let _private_key = PrivateKey::new(None, rng).unwrap();
        });
    });

    let private_key = PrivateKey::new(None, rng).unwrap();

    group.bench_function("public_key", |b| {
        b.iter(|| {
            let _public_key = PublicKey::from(&private_key).unwrap();
        });
    });
}

criterion_group!(benches, account_bench);

criterion_main!(benches);
