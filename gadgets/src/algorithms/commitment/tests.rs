use super::*;
use crate::curves::edwards_bls12::EdwardsBlsGadget;
use snarkos_algorithms::{
    commitment::{Blake2sCommitment, PedersenCommitment},
    crh::PedersenSize,
};
use snarkos_curves::edwards_bls12::{EdwardsProjective, Fq, Fr};
use snarkos_models::{
    algorithms::CommitmentScheme,
    curves::ProjectiveCurve,
    gadgets::{
        algorithms::CommitmentGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
};
use snarkos_utilities::rand::UniformRand;

use rand::{thread_rng, Rng};

#[test]
fn blake2s_commitment_gadget_test() {
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
        assert_eq!(native_result[i], gadget_result.0[i].value.unwrap());
    }
    assert!(cs.is_satisfied());
}

#[test]
fn pedersen_commitment_gadget_test() {
    let mut cs = TestConstraintSystem::<Fq>::new();

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(super) struct Size;

    impl PedersenSize for Size {
        const NUM_WINDOWS: usize = 8;
        const WINDOW_SIZE: usize = 4;
    }

    type TestCommitment = PedersenCommitment<EdwardsProjective, Size>;
    type TestCommitmentGadget = PedersenCommitmentGadget<EdwardsProjective, Fq, EdwardsBlsGadget>;

    let rng = &mut thread_rng();

    let input = [1u8; 4];
    let randomness = Fr::rand(rng);
    let commitment = PedersenCommitment::<EdwardsProjective, Size>::setup(rng);
    let native_output = commitment.commit(&input, &randomness).unwrap();

    let mut input_bytes = vec![];
    for (byte_i, input_byte) in input.iter().enumerate() {
        let cs = cs.ns(|| format!("input_byte_gadget_{}", byte_i));
        input_bytes.push(UInt8::alloc(cs, || Ok(*input_byte)).unwrap());
    }

    let randomness_gadget = <TestCommitmentGadget as CommitmentGadget<TestCommitment, Fq>>::RandomnessGadget::alloc(
        &mut cs.ns(|| "randomness_gadget"),
        || Ok(&randomness),
    )
    .unwrap();
    let parameters_gadget = <TestCommitmentGadget as CommitmentGadget<TestCommitment, Fq>>::ParametersGadget::alloc(
        &mut cs.ns(|| "parameters_gadget"),
        || Ok(&commitment.parameters),
    )
    .unwrap();
    let output_gadget = <TestCommitmentGadget as CommitmentGadget<TestCommitment, Fq>>::check_commitment_gadget(
        &mut cs.ns(|| "commitment_gadget"),
        &parameters_gadget,
        &input_bytes,
        &randomness_gadget,
    )
    .unwrap();

    let native_output = native_output.into_affine();
    assert_eq!(native_output.x, output_gadget.x.value.unwrap());
    assert_eq!(native_output.y, output_gadget.y.value.unwrap());
    assert!(cs.is_satisfied());
}
