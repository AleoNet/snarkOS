mod pairing {
    use snarkos_curves::{
        bw6_761::{BW6_761, BW6_761Parameters, Fq6, G1Affine, G1Projective as G1, G2Affine, G2Projective as G2},
        templates::bw6::{G1Prepared, G2Prepared},
    };
    use snarkos_models::curves::{PairingCurve, PairingEngine};
    use snarkos_utilities::rand::UniformRand;

    use rand::SeedableRng;
    use rand_xorshift::XorShiftRng;

    #[bench]
    fn bench_pairing_miller_loop(b: &mut ::test::Bencher) {
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
        b.iter(|| {
            let tmp = BW6_761::miller_loop(&[(&v[count].0, &v[count].1)]);
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
            .map(|(ref p, ref q)| BW6_761::miller_loop(&[(p, q)]))
            .collect();

        let mut count = 0;
        b.iter(|| {
            let tmp = BW6_761::final_exponentiation(&v[count]);
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
            let tmp = BW6_761::pairing(v[count].0, v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        });
    }
}
