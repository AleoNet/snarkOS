use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::Field,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

struct MySillyCircuit<F: Field> {
    a: Option<F>,
    b: Option<F>,
}

impl<F: Field> ConstraintSynthesizer<F> for MySillyCircuit<F> {
    fn generate_constraints<CS: ConstraintSystem<F>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
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

        Ok(())
    }
}

mod bls12_377 {
    use super::*;
    use crate::snark::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};
    use snarkos_curves::bls12_377::{Bls12_377, Fr};
    use snarkos_utilities::rand::{test_rng, UniformRand};

    use std::ops::MulAssign;

    #[test]
    fn prove_and_verify() {
        let rng = &mut test_rng();

        let params = generate_random_parameters::<Bls12_377, _, _>(MySillyCircuit { a: None, b: None }, rng).unwrap();

        let pvk = prepare_verifying_key::<Bls12_377>(&params.vk);

        for _ in 0..10 {
            let a = Fr::rand(rng);
            let b = Fr::rand(rng);
            let mut c = a;
            c.mul_assign(&b);

            let proof = create_random_proof(MySillyCircuit { a: Some(a), b: Some(b) }, &params, rng).unwrap();

            assert!(verify_proof(&pvk, &proof, &[c]).unwrap());
            assert!(!verify_proof(&pvk, &proof, &[a]).unwrap());
        }
    }
}

mod sw6 {
    use super::*;
    use crate::snark::{create_random_proof, generate_random_parameters, prepare_verifying_key, verify_proof};
    use snarkos_curves::sw6::{Fr as SW6Fr, SW6};
    use snarkos_models::curves::Field;
    use snarkos_utilities::rand::{test_rng, UniformRand};

    #[test]
    fn prove_and_verify() {
        let rng = &mut test_rng();

        let params = generate_random_parameters::<SW6, _, _>(MySillyCircuit { a: None, b: None }, rng).unwrap();

        let pvk = prepare_verifying_key::<SW6>(&params.vk);

        let a = SW6Fr::rand(rng);
        let b = SW6Fr::rand(rng);
        let c = a * &b;

        let proof = create_random_proof(MySillyCircuit { a: Some(a), b: Some(b) }, &params, rng).unwrap();

        assert!(verify_proof(&pvk, &proof, &[c]).unwrap());
        assert!(!verify_proof(&pvk, &proof, &[SW6Fr::zero()]).unwrap());
    }
}

mod gm17 {
    use super::*;

    use rand::thread_rng;
    use std::ops::AddAssign;

    #[test]
    fn test_gm17() {
        use crate::snark::GM17;
        use snarkos_curves::bls12_377::{Bls12_377, Fr};
        use snarkos_models::algorithms::SNARK;

        #[derive(Copy, Clone)]
        struct R1CSCircuit {
            x: Option<Fr>,
            sum: Option<Fr>,
            w: Option<Fr>,
        }

        impl R1CSCircuit {
            pub(super) fn new(x: Fr, sum: Fr, w: Fr) -> Self {
                Self {
                    x: Some(x),
                    sum: Some(sum),
                    w: Some(w),
                }
            }
        }

        impl ConstraintSynthesizer<Fr> for R1CSCircuit {
            fn generate_constraints<CS: ConstraintSystem<Fr>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
                let input = cs.alloc_input(|| "x", || Ok(self.x.unwrap()))?;
                let sum = cs.alloc_input(|| "sum", || Ok(self.sum.unwrap()))?;
                let witness = cs.alloc(|| "w", || Ok(self.w.unwrap()))?;

                cs.enforce(
                    || "check_one",
                    |lc| lc + sum,
                    |lc| lc + CS::one(),
                    |lc| lc + input + witness,
                );
                Ok(())
            }
        }

        let mut sum = Fr::one();
        sum.add_assign(&Fr::one());
        let circuit = R1CSCircuit::new(Fr::one(), sum, Fr::one());

        let rng = &mut thread_rng();

        let parameters = GM17::<Bls12_377, R1CSCircuit, [Fr]>::setup(circuit, rng).unwrap();

        let proof = GM17::<Bls12_377, R1CSCircuit, [Fr]>::prove(&parameters.0, circuit, rng).unwrap();

        let result = GM17::<Bls12_377, R1CSCircuit, [Fr]>::verify(&parameters.1, &[Fr::one(), sum], &proof).unwrap();
        assert!(result);
    }
}

mod serialization {
    use super::*;
    use crate::snark::{create_random_proof, generate_random_parameters, Parameters, Proof, VerifyingKey};

    use snarkos_curves::bls12_377::{Bls12_377, Fr};
    use snarkos_models::storage::Storage;
    use snarkos_utilities::{
        bytes::{FromBytes, ToBytes},
        rand::UniformRand,
        to_bytes,
    };

    use rand::thread_rng;

    const TEST_PARAMETERS_PATH: &str = "./snark.params";

    #[test]
    fn proof_serialization() {
        let rng = &mut thread_rng();

        let parameters =
            generate_random_parameters::<Bls12_377, _, _>(MySillyCircuit { a: None, b: None }, rng).unwrap();

        let a = Fr::rand(rng);
        let b = Fr::rand(rng);

        let proof = create_random_proof(MySillyCircuit { a: Some(a), b: Some(b) }, &parameters, rng).unwrap();

        let proof_bytes = to_bytes![proof].unwrap();
        let recovered_proof: Proof<Bls12_377> = FromBytes::read(&proof_bytes[..]).unwrap();

        assert_eq!(proof, recovered_proof);
    }

    #[test]
    fn parameter_serialization() {
        let rng = &mut thread_rng();

        let parameters =
            generate_random_parameters::<Bls12_377, _, _>(MySillyCircuit { a: None, b: None }, rng).unwrap();
        let vk = parameters.vk.clone();

        let parameter_bytes = to_bytes![&parameters].unwrap();
        let vk_bytes = to_bytes![&vk].unwrap();

        let recovered_parameters: Parameters<Bls12_377> = FromBytes::read(&parameter_bytes[..]).unwrap();
        let recovered_vk: VerifyingKey<Bls12_377> = FromBytes::read(&vk_bytes[..]).unwrap();

        assert_eq!(parameters, recovered_parameters);
        assert_eq!(vk, recovered_vk);
    }

    #[test]
    fn parameter_storage() {
        let rng = &mut thread_rng();
        let mut path = std::env::temp_dir();
        path.push(TEST_PARAMETERS_PATH);

        let parameters =
            generate_random_parameters::<Bls12_377, _, _>(MySillyCircuit { a: None, b: None }, rng).unwrap();
        parameters.store(&path).unwrap();

        let recovered_parameters = Parameters::<Bls12_377>::load(&path).unwrap();

        assert_eq!(parameters, recovered_parameters);

        std::fs::remove_file(&path).unwrap();
    }
}
