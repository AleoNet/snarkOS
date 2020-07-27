use criterion::{criterion_group, criterion_main, Criterion};
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use snarkos_curves::bls12_377::Bls12_377;
use snarkos_models::algorithms::snark::SNARK;
use snarkos_posw::{txids_to_roots, Marlin, Posw, PoswMarlin, GM17};
use snarkos_utilities::bytes::FromBytes;

fn gm17_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof of Succinct Work: GM17");
    group.sample_size(10);
    let rng = &mut XorShiftRng::seed_from_u64(1234567);

    // run the setup
    let posw = Posw::setup(rng).unwrap();

    // super low difficulty so we find a solution immediately
    let difficulty_target = 0xFFFF_FFFF_FFFF_FFFF_u64;

    // Can we test for different tx id sizes?
    let transaction_ids = vec![vec![1u8; 32]; 8];
    let (_, pedersen_merkle_root, subroots) = txids_to_roots(&transaction_ids);

    // Proof Generation Bench
    group.bench_function("mine", |b| {
        b.iter(|| {
            let (_nonce, _proof) = posw.mine(&subroots, difficulty_target, rng, std::u32::MAX).unwrap();
        });
    });

    let (nonce, proof) = posw.mine(&subroots, difficulty_target, rng, std::u32::MAX).unwrap();
    let proof = <GM17<Bls12_377> as SNARK>::Proof::read(&proof[..]).unwrap();

    group.bench_function("verify", |b| {
        b.iter(|| {
            let _ = posw.verify(nonce, &proof, &pedersen_merkle_root).unwrap();
        });
    });
}

fn marlin_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof of Succinct Work: Marlin");
    group.sample_size(10);
    let rng = &mut XorShiftRng::seed_from_u64(1234567);

    let universal_srs = snarkos_marlin::snark::Marlin::<Bls12_377>::universal_setup(10000, 10000, 100000, rng).unwrap();
    let posw = PoswMarlin::index(universal_srs).unwrap();

    let difficulty_target = 0xFFFF_FFFF_FFFF_FFFF_u64;

    let transaction_ids = vec![vec![1u8; 32]; 8];
    let (_, pedersen_merkle_root, subroots) = txids_to_roots(&transaction_ids);

    // Proof Generation Bench
    group.bench_function("mine", |b| {
        b.iter(|| {
            let (_nonce, _proof) = posw.mine(&subroots, difficulty_target, rng, std::u32::MAX).unwrap();
        });
    });

    let (nonce, proof) = posw.mine(&subroots, difficulty_target, rng, std::u32::MAX).unwrap();
    let proof = <Marlin<Bls12_377> as SNARK>::Proof::read(&proof[..]).unwrap();

    group.bench_function("verify", |b| {
        b.iter(|| {
            let _ = posw.verify(nonce, &proof, &pedersen_merkle_root).unwrap();
        });
    });
}

criterion_group!(benches, gm17_bench, marlin_bench);

criterion_main!(benches);
