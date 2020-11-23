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

pub(crate) mod pairing {
    use snarkos_curves::{
        bw6_761::{BW6_761Parameters, Fq6, G1Affine, G1Projective as G1, G2Affine, G2Projective as G2, BW6_761},
        templates::bw6::{G1Prepared, G2Prepared},
    };
    use snarkos_models::curves::{PairingCurve, PairingEngine};
    use snarkos_utilities::rand::UniformRand;

    use criterion::Criterion;
    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    use std::iter;

    pub fn bench_pairing_miller_loop(c: &mut Criterion) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1Prepared<BW6_761Parameters>, G2Prepared<BW6_761Parameters>)> = (0..SAMPLES)
            .map(|_| {
                (
                    G1Affine::from(G1::rand(&mut rng)).prepare(),
                    G2Affine::from(G2::rand(&mut rng)).prepare(),
                )
            })
            .collect();

        let mut count = 0;
        c.bench_function("bw6_761: pairing_miller_loop", |c| {
            c.iter(|| {
                let tmp = BW6_761::miller_loop(iter::once((&v[count].0, &v[count].1)));
                count = (count + 1) % SAMPLES;
                tmp
            })
        });
    }

    pub fn bench_pairing_final_exponentiation(c: &mut Criterion) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<Fq6> = (0..SAMPLES)
            .map(|_| {
                (
                    G1Affine::from(G1::rand(&mut rng)).prepare(),
                    G2Affine::from(G2::rand(&mut rng)).prepare(),
                )
            })
            .map(|(ref p, ref q)| BW6_761::miller_loop([(p, q)].iter().copied()))
            .collect();

        let mut count = 0;
        c.bench_function("bw6_761: pairing_final_exponentiation", |c| {
            c.iter(|| {
                let tmp = BW6_761::final_exponentiation(&v[count]);
                count = (count + 1) % SAMPLES;
                tmp
            })
        });
    }

    pub fn bench_pairing_full(c: &mut Criterion) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1, G2)> = (0..SAMPLES).map(|_| (G1::rand(&mut rng), G2::rand(&mut rng))).collect();

        let mut count = 0;
        c.bench_function("bw6_761: pairing_full", |c| {
            c.iter(|| {
                let tmp = BW6_761::pairing(v[count].0, v[count].1);
                count = (count + 1) % SAMPLES;
                tmp
            })
        });
    }
}
