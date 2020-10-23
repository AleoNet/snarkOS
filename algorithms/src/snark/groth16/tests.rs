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

use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Zero},
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

struct MySillyCircuit<F: Field> {
    a: Option<F>,
    b: Option<F>,
}

impl<ConstraintF: Field> ConstraintSynthesizer<ConstraintF> for MySillyCircuit<ConstraintF> {
    fn generate_constraints<CS: ConstraintSystem<ConstraintF>>(&self, cs: &mut CS) -> Result<(), SynthesisError> {
        let a = cs.alloc(|| "a", || self.a.ok_or(SynthesisError::AssignmentMissing))?;
        let b = cs.alloc(|| "b", || self.b.ok_or(SynthesisError::AssignmentMissing))?;
        let c = cs.alloc_input(
            || "c",
            || {
                let mut a = self.a.ok_or(SynthesisError::AssignmentMissing)?;
                let b = self.b.ok_or(SynthesisError::AssignmentMissing)?;

                a.mul_assign(&b);
                Ok(a)
            },
        )?;

        cs.enforce(|| "a*b=c", |lc| lc + a, |lc| lc + b, |lc| lc + c);
        cs.enforce(|| "a*b=c", |lc| lc + a, |lc| lc + b, |lc| lc + c);
        cs.enforce(|| "a*b=c", |lc| lc + a, |lc| lc + b, |lc| lc + c);
        cs.enforce(|| "a*b=c", |lc| lc + a, |lc| lc + b, |lc| lc + c);
        cs.enforce(|| "a*b=c", |lc| lc + a, |lc| lc + b, |lc| lc + c);
        cs.enforce(|| "a*b=c", |lc| lc + a, |lc| lc + b, |lc| lc + c);

        Ok(())
    }
}

mod bls12_377 {
    use super::*;
    use crate::snark::groth16::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};
    use core::ops::MulAssign;
    use snarkos_curves::bls12_377::{Bls12_377, Fr};
    use snarkos_utilities::rand::{test_rng, UniformRand};

    #[test]
    fn prove_and_verify() {
        let rng = &mut test_rng();

        let params = generate_random_parameters::<Bls12_377, _, _>(&MySillyCircuit { a: None, b: None }, rng).unwrap();

        let pvk = prepare_verifying_key::<Bls12_377>(&params.vk);

        for _ in 0..100 {
            let a = Fr::rand(rng);
            let b = Fr::rand(rng);
            let mut c = a;
            c.mul_assign(&b);

            let proof = create_random_proof(&MySillyCircuit { a: Some(a), b: Some(b) }, &params, rng).unwrap();

            assert!(verify_proof(&pvk, &proof, &[c]).unwrap());
            assert!(!verify_proof(&pvk, &proof, &[a]).unwrap());
        }
    }
}

mod bw6_761 {
    use super::*;
    use crate::snark::groth16::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};

    use snarkos_curves::bw6_761::{Fr, BW6_761};
    use snarkos_utilities::rand::{test_rng, UniformRand};

    #[test]
    fn prove_and_verify() {
        let rng = &mut test_rng();

        let params = generate_random_parameters::<BW6_761, _, _>(&MySillyCircuit { a: None, b: None }, rng).unwrap();

        let pvk = prepare_verifying_key::<BW6_761>(&params.vk);

        let a = Fr::rand(rng);
        let b = Fr::rand(rng);
        let c = a * &b;

        let proof = create_random_proof(&MySillyCircuit { a: Some(a), b: Some(b) }, &params, rng).unwrap();

        assert!(verify_proof(&pvk, &proof, &[c]).unwrap());
        assert!(!verify_proof(&pvk, &proof, &[Fr::zero()]).unwrap());
    }
}
