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

use crate::fft::{domain::*, multicore::*};
use snarkos_curves::bls12_377::Bls12_377;
use snarkos_models::curves::PairingEngine;
use snarkos_utilities::rand::UniformRand;

use rand;
use std::cmp::min;

// Test multiplying various (low degree) polynomials together and
// comparing with naive evaluations.
#[test]
fn fft_composition() {
    fn test_fft_composition<E: PairingEngine, R: rand::Rng>(rng: &mut R) {
        for coeffs in 0..10 {
            let coeffs = 1 << coeffs;

            let mut v = vec![];
            for _ in 0..coeffs {
                v.push(E::Fr::rand(rng));
            }
            let mut v2 = v.clone();

            let domain = EvaluationDomain::<E::Fr>::new(coeffs).unwrap();
            domain.ifft_in_place(&mut v2);
            domain.fft_in_place(&mut v2);
            assert_eq!(v, v2, "ifft(fft(.)) != iden");

            domain.fft_in_place(&mut v2);
            domain.ifft_in_place(&mut v2);
            assert_eq!(v, v2, "fft(ifft(.)) != iden");

            domain.coset_ifft_in_place(&mut v2);
            domain.coset_fft_in_place(&mut v2);
            assert_eq!(v, v2, "coset_ifft(coset_fft(.)) != iden");

            domain.coset_fft_in_place(&mut v2);
            domain.coset_ifft_in_place(&mut v2);
            assert_eq!(v, v2, "coset_ifft(coset_fft(.)) != iden");
        }
    }

    let rng = &mut rand::thread_rng();

    test_fft_composition::<Bls12_377, _>(rng);
}

#[test]
fn parallel_fft_consistency() {
    fn test_consistency<E: PairingEngine, R: rand::Rng>(rng: &mut R) {
        let worker = Worker::new();

        for _ in 0..5 {
            for log_d in 0..10 {
                let d = 1 << log_d;

                let mut v1 = (0..d).map(|_| E::Fr::rand(rng)).collect::<Vec<_>>();
                let mut v2 = v1.clone();

                let domain = EvaluationDomain::new(v1.len()).unwrap();

                for log_cpus in log_d..min(log_d + 1, 3) {
                    parallel_fft(&mut v1, &worker, domain.group_gen, log_d, log_cpus);
                    serial_fft(&mut v2, domain.group_gen, log_d);

                    assert_eq!(v1, v2);
                }
            }
        }
    }

    let rng = &mut rand::thread_rng();

    test_consistency::<Bls12_377, _>(rng);
}
