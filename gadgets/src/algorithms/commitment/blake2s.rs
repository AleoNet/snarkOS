use crate::algorithms::prf::{blake2s_gadget, Blake2sOutputGadget};
use snarkos_algorithms::commitment::Blake2sCommitment;
use snarkos_models::{
    curves::{Field, PrimeField},
    gadgets::{
        algorithms::CommitmentGadget,
        r1cs::{ConstraintSystem, SynthesisError},
        utilities::{alloc::AllocGadget, uint8::UInt8, ToBytesGadget},
    },
};

use std::borrow::Borrow;

#[derive(Clone)]
pub struct Blake2sCommitmentGadget;

impl<F: PrimeField> CommitmentGadget<Blake2sCommitment, F> for Blake2sCommitmentGadget {
    type OutputGadget = Blake2sOutputGadget;
    type ParametersGadget = Blake2sParametersGadget;
    type RandomnessGadget = Blake2sRandomnessGadget;

    fn check_commitment_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        _: &Self::ParametersGadget,
        input: &[UInt8],
        r: &Self::RandomnessGadget,
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let mut input_bits = vec![];
        for byte in input.iter().chain(r.0.iter()) {
            input_bits.extend_from_slice(&byte.into_bits_le());
        }

        let mut result = vec![];
        for (i, int) in blake2s_gadget(cs.ns(|| "blake2s_commitment"), &input_bits)?
            .into_iter()
            .enumerate()
        {
            result.extend_from_slice(&int.to_bytes(&mut cs.ns(|| format!("to_bytes_{}", i)))?);
        }
        Ok(Blake2sOutputGadget(result))
    }
}

#[derive(Clone)]
pub struct Blake2sParametersGadget;

impl<F: Field> AllocGadget<(), F> for Blake2sParametersGadget {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<()>, CS: ConstraintSystem<F>>(
        _: CS,
        _: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Blake2sParametersGadget)
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<()>, CS: ConstraintSystem<F>>(
        _: CS,
        _: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Blake2sParametersGadget)
    }
}

#[derive(Clone)]
pub struct Blake2sRandomnessGadget(pub Vec<UInt8>);

impl<F: PrimeField> AllocGadget<[u8; 32], F> for Blake2sRandomnessGadget {
    #[inline]
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[u8; 32]>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Blake2sRandomnessGadget(<UInt8>::alloc_vec(cs, &match value_gen() {
            Ok(val) => *(val.borrow()),
            Err(_) => [0u8; 32],
        })?))
    }

    #[inline]
    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[u8; 32]>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Blake2sRandomnessGadget(<UInt8>::alloc_input_vec(
            cs,
            &match value_gen() {
                Ok(val) => *(val.borrow()),
                Err(_) => [0u8; 32],
            },
        )?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use snarkos_algorithms::commitment::Blake2sCommitment;
    use snarkos_curves::bls12_377::Fr;
    use snarkos_models::{algorithms::CommitmentScheme, gadgets::r1cs::TestConstraintSystem};

    use rand::{thread_rng, Rng};

    #[test]
    fn commitment_gadget_test() {
        let mut cs = TestConstraintSystem::<Fr>::new();
        let rng = &mut thread_rng();

        let input = [1u8; 32];

        let mut randomness = [0u8; 32];
        rng.fill(&mut randomness);

        let commitment = Blake2sCommitment::setup(rng);
        let native_result = commitment.commit(&input, &randomness).unwrap();

        let mut input_bytes = vec![];
        for (byte_i, input_byte) in input.iter().enumerate() {
            let cs = cs.ns(|| format!("input_byte_gadget_{}", byte_i));
            input_bytes.push(UInt8::alloc(cs, || Ok(*input_byte)).unwrap());
        }

        let mut randomness_bytes = vec![];
        for (byte_i, random_byte) in randomness.iter().enumerate() {
            let cs = cs.ns(|| format!("randomness_byte_gadget_{}", byte_i));
            randomness_bytes.push(UInt8::alloc(cs, || Ok(*random_byte)).unwrap());
        }
        let randomness_bytes = Blake2sRandomnessGadget(randomness_bytes);

        let gadget_parameters = Blake2sParametersGadget::alloc(&mut cs.ns(|| "gadget_parameters"), || Ok(&())).unwrap();
        let gadget_result = Blake2sCommitmentGadget::check_commitment_gadget(
            &mut cs.ns(|| "gadget_evaluation"),
            &gadget_parameters,
            &input_bytes,
            &randomness_bytes,
        )
        .unwrap();

        for i in 0..32 {
            assert_eq!(native_result[i], gadget_result.0[i].get_value().unwrap());
        }
        assert!(cs.is_satisfied());
    }
}
