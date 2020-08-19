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

mod pairing {
    use algebra::{
        curves::{
            sw6::{G1Affine, G1Projective as G1, G2Affine, G2Projective as G2, SW6},
            PairingCurve,
            PairingEngine,
        },
        fields::sw6::Fq6,
        UniformRand,
    };
    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    #[bench]
    fn bench_pairing_miller_loop(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1Affine, G2Affine)> = (0..SAMPLES)
            .map(|_| {
                (
                    G1Affine::from(G1::rand(&mut rng)).prepare(),
                    G2Affine::from(G2::rand(&mut rng)).prepare(),
                )
            })
            .collect();

        let mut count = 0;
        b.iter(|| {
            let tmp = SW6::miller_loop(&[(&v[count].0, &v[count].1)]);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_pairing_final_exponentiation(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<Fq6> = (0..SAMPLES)
            .map(|_| {
                (
                    G1Affine::from(G1::rand(&mut rng)).prepare(),
                    G2Affine::from(G2::rand(&mut rng)).prepare(),
                )
            })
            .map(|(ref p, ref q)| SW6::miller_loop(&[(p, q)]))
            .collect();

        let mut count = 0;
        b.iter(|| {
            let tmp = SW6::final_exponentiation(&v[count]);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_pairing_full(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1, G2)> = (0..SAMPLES).map(|_| (G1::rand(&mut rng), G2::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let tmp = SW6::pairing(v[count].0, v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }
}
