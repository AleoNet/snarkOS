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

use snarkvm_curves::bls12_377::Bls12_377;
use snarkvm_algorithms::snark::SNARK;
use snarkvm_posw::txids_to_roots;
use snarkvm_posw::Marlin;
use snarkvm_posw::Posw;
use snarkvm_posw::PoswMarlin;
use snarkvm_posw::GM17;
use snarkvm_utilities::bytes::FromBytes;

use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

fn gm17_bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("Proof of Succinct Work: GM17");
    group.sample_size(10);
    let rng = &mut XorShiftRng::seed_from_u64(1234567);

    // run the setup
    let posw = Posw::setup(rng).unwrap();

    // super low difficulty so we find a solution immediately
    let difficulty_target = 0xFFFF_FFFF_FFFF_FFFF_u64;

    // Can we test for different tx id sizes?
    let transaction_ids = vec![[1u8; 32]; 8];
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

    let universal_srs = snarkvm_marlin::snark::Marlin::<Bls12_377>::universal_setup(10000, 10000, 100000, rng).unwrap();
    let posw = PoswMarlin::index(universal_srs).unwrap();

    let difficulty_target = 0xFFFF_FFFF_FFFF_FFFF_u64;

    let transaction_ids = vec![[1u8; 32]; 8];
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
