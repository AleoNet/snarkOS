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

mod g1 {
    use algebra::{
        curves::sw6::{G1Affine, G1Projective as G1},
        fields::sw6::Fr,
        ProjectiveCurve,
        UniformRand,
    };
    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;
    use std::ops::AddAssign;

    #[bench]
    fn bench_g1_rand(b: &mut ::test::Bencher) {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
        b.iter(|| G1::rand(&mut rng));
    }

    #[bench]
    fn bench_g1_mul_assign(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1, Fr)> = (0..SAMPLES).map(|_| (G1::rand(&mut rng), Fr::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.mul_assign(v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_g1_add_assign(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1, G1)> = (0..SAMPLES).map(|_| (G1::rand(&mut rng), G1::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.add_assign(&v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_g1_add_assign_mixed(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1, G1Affine)> = (0..SAMPLES)
            .map(|_| (G1::rand(&mut rng), G1::rand(&mut rng).into()))
            .collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.add_assign_mixed(&v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_g1_double(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G1, G1)> = (0..SAMPLES).map(|_| (G1::rand(&mut rng), G1::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.double_in_place();
            count = (count + 1) % SAMPLES;
            tmp
        });
    }
}

mod g2 {
    use algebra::{
        curves::sw6::{G2Affine, G2Projective as G2},
        fields::sw6::Fr,
        ProjectiveCurve,
        UniformRand,
    };
    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;
    use std::ops::AddAssign;

    #[bench]
    fn bench_g2_rand(b: &mut ::test::Bencher) {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
        b.iter(|| G2::rand(&mut rng));
    }

    #[bench]
    fn bench_g2_mul_assign(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G2, Fr)> = (0..SAMPLES).map(|_| (G2::rand(&mut rng), Fr::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.mul_assign(v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_g2_add_assign(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G2, G2)> = (0..SAMPLES).map(|_| (G2::rand(&mut rng), G2::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.add_assign(&v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_g2_add_assign_mixed(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G2, G2Affine)> = (0..SAMPLES)
            .map(|_| (G2::rand(&mut rng), G2::rand(&mut rng).into()))
            .collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.add_assign_mixed(&v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }

    #[bench]
    fn bench_g2_double(b: &mut ::test::Bencher) {
        const SAMPLES: usize = 1000;

        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let v: Vec<(G2, G2)> = (0..SAMPLES).map(|_| (G2::rand(&mut rng), G2::rand(&mut rng))).collect();

        let mut count = 0;
        b.iter(|| {
            let mut tmp = v[count].0;
            tmp.double_in_place();
            count = (count + 1) % SAMPLES;
            tmp
        });
    }
}
