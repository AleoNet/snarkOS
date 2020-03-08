use crate::{algorithms::crh::PedersenCRHGadget, curves::edwards_bls12::EdwardsBlsGadget};
use snarkos_algorithms::crh::{PedersenCRH, PedersenSize};
use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsProjective};
use snarkos_models::{
    algorithms::CRH,
    curves::ProjectiveCurve,
    gadgets::{
        algorithms::CRHGadget,
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
};

use rand::{thread_rng, Rng};

type TestCRH = PedersenCRH<EdwardsProjective, Window>;
type TestCRHGadget = PedersenCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) struct Window;

impl PedersenSize for Window {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 128;
}

fn generate_input<CS: ConstraintSystem<Fr>, R: Rng>(mut cs: CS, rng: &mut R) -> ([u8; 128], Vec<UInt8>) {
    let mut input = [1u8; 128];
    rng.fill_bytes(&mut input);

    let mut input_bytes = vec![];
    for (byte_i, input_byte) in input.iter().enumerate() {
        let cs = cs.ns(|| format!("input_byte_gadget_{}", byte_i));
        input_bytes.push(UInt8::alloc(cs, || Ok(*input_byte)).unwrap());
    }
    (input, input_bytes)
}

#[test]
fn crh_primitive_gadget_test() {
    let rng = &mut thread_rng();
    let mut cs = TestConstraintSystem::<Fr>::new();

    let (input, input_bytes) = generate_input(&mut cs, rng);
    println!("number of constraints for input: {}", cs.num_constraints());

    let crh = TestCRH::setup(rng);
    let native_result = crh.hash(&input).unwrap();

    let parameters_gadget = <TestCRHGadget as CRHGadget<TestCRH, Fr>>::ParametersGadget::alloc(
        &mut cs.ns(|| "gadget_parameters"),
        || Ok(&crh.parameters),
    )
        .unwrap();
    println!("number of constraints for input + params: {}", cs.num_constraints());

    let output_gadget = <TestCRHGadget as CRHGadget<TestCRH, Fr>>::check_evaluation_gadget(
        &mut cs.ns(|| "gadget_evaluation"),
        &parameters_gadget,
        &input_bytes,
    )
        .unwrap();

    println!("number of constraints total: {}", cs.num_constraints());

    let native_result = native_result.into_affine();
    assert_eq!(native_result.x, output_gadget.x.value.unwrap());
    assert_eq!(native_result.y, output_gadget.y.value.unwrap());
    assert!(cs.is_satisfied());
}
