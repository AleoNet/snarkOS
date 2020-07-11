use crate::{
    algorithms::crh::{
        BoweHopwoodPedersenCRHGadget,
        BoweHopwoodPedersenCompressedCRHGadget,
        PedersenCRHGadget,
        PedersenCompressedCRHGadget,
    },
    curves::edwards_bls12::EdwardsBlsGadget,
};
use snarkos_algorithms::crh::{
    BoweHopwoodPedersenCRH,
    BoweHopwoodPedersenCompressedCRH,
    PedersenCRH,
    PedersenCompressedCRH,
    PedersenSize,
};
use snarkos_curves::{
    bls12_377::Fr,
    edwards_bls12::{EdwardsAffine, EdwardsProjective},
};
use snarkos_models::{
    algorithms::{CRHParameters, CRH},
    curves::{Field, PrimeField},
    gadgets::{
        algorithms::{CRHGadget, MaskedCRHGadget},
        r1cs::{ConstraintSystem, TestConstraintSystem},
        utilities::{alloc::AllocGadget, eq::EqGadget, uint::UInt8},
    },
};

use rand::{thread_rng, Rng};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Size;

impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 8;
    const WINDOW_SIZE: usize = 128;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct BoweHopwoodSize;

impl PedersenSize for BoweHopwoodSize {
    const NUM_WINDOWS: usize = 32;
    const WINDOW_SIZE: usize = 48;
}

const PEDERSEN_HASH_CONSTRAINTS: usize = 5632;
const PEDERSEN_HASH_CONSTRAINTS_ON_AFFINE: usize = 6656;
const BOWE_HOPWOOD_HASH_CONSTRAINTS: usize = 3974;

fn generate_input<F: Field, CS: ConstraintSystem<F>, R: Rng>(
    mut cs: CS,
    rng: &mut R,
) -> ([u8; 128], Vec<UInt8>, Vec<UInt8>) {
    let mut input = [1u8; 128];
    rng.fill_bytes(&mut input);
    let mut mask = [1u8; 128];
    rng.fill_bytes(&mut mask);

    let mut input_bytes = vec![];
    let mut mask_bytes = vec![];
    for (byte_i, (input_byte, mask_byte)) in input.iter().zip(mask.iter()).enumerate() {
        let cs_input = cs.ns(|| format!("input_byte_gadget_{}", byte_i));
        input_bytes.push(UInt8::alloc(cs_input, || Ok(*input_byte)).unwrap());
        // The mask will later on be extended to be double the size, so we only need half the bits
        // as the input.
        if byte_i % 2 == 0 {
            let cs_mask = cs.ns(|| format!("mask_byte_gadget_{}", byte_i));
            mask_bytes.push(UInt8::alloc(cs_mask, || Ok(*mask_byte)).unwrap());
        }
    }
    (input, input_bytes, mask_bytes)
}

fn primitive_crh_gadget_test<F: Field, H: CRH, CG: CRHGadget<H, F>>(hash_constraints: usize) {
    let rng = &mut thread_rng();
    let mut cs = TestConstraintSystem::<F>::new();

    let (input, input_bytes, _mask_bytes) = generate_input(&mut cs, rng);
    assert_eq!(cs.num_constraints(), 1536);

    let crh = H::setup(rng);
    let native_result = crh.hash(&input).unwrap();

    let parameters_gadget =
        <CG as CRHGadget<_, _>>::ParametersGadget::alloc(&mut cs.ns(|| "gadget_parameters"), || Ok(crh.parameters()))
            .unwrap();
    assert_eq!(cs.num_constraints(), 1536);

    let output_gadget = <CG as CRHGadget<_, _>>::check_evaluation_gadget(
        &mut cs.ns(|| "gadget_evaluation"),
        &parameters_gadget,
        &input_bytes,
    )
    .unwrap();
    assert_eq!(cs.num_constraints(), hash_constraints);

    let native_result_gadget =
        <CG as CRHGadget<_, _>>::OutputGadget::alloc(&mut cs.ns(|| "native_result"), || Ok(&native_result)).unwrap();

    output_gadget
        .enforce_equal(
            &mut cs.ns(|| "Check that computed crh matches provided output"),
            &native_result_gadget,
        )
        .unwrap();

    assert!(cs.is_satisfied());
}

fn masked_crh_gadget_test<F: PrimeField, H: CRH, CG: MaskedCRHGadget<H, F>>() {
    let rng = &mut thread_rng();
    let mut cs = TestConstraintSystem::<F>::new();

    let (input, input_bytes, mask_bytes) = generate_input(&mut cs, rng);
    assert_eq!(cs.num_constraints(), 1536);

    let crh = H::setup(rng);
    let mask_parameters = H::Parameters::setup(rng);
    let native_result = crh.hash(&input).unwrap();

    let parameters_gadget =
        <CG as CRHGadget<_, _>>::ParametersGadget::alloc(&mut cs.ns(|| "gadget_parameters"), || Ok(crh.parameters()))
            .unwrap();
    assert_eq!(cs.num_constraints(), 1536);

    let mask_parameters_gadget =
        <CG as CRHGadget<_, _>>::ParametersGadget::alloc(&mut cs.ns(|| "gadget_mask_parameters"), || {
            Ok(mask_parameters)
        })
        .unwrap();
    assert_eq!(cs.num_constraints(), 1536);

    let masked_output_gadget = <CG as MaskedCRHGadget<_, _>>::check_evaluation_gadget_masked(
        &mut cs.ns(|| "masked_gadget_evaluation"),
        &parameters_gadget,
        &input_bytes,
        &mask_parameters_gadget,
        &mask_bytes,
    )
    .unwrap();
    assert_eq!(cs.num_constraints(), 17932);

    let native_result_gadget =
        <CG as CRHGadget<_, _>>::OutputGadget::alloc(&mut cs.ns(|| "native_result"), || Ok(&native_result)).unwrap();

    masked_output_gadget
        .enforce_equal(
            &mut cs.ns(|| "Check that computed crh matches provided output"),
            &native_result_gadget,
        )
        .unwrap();

    assert!(cs.is_satisfied());
}

mod pedersen_crh_gadget_on_projective {
    use super::*;

    type TestCRH = PedersenCRH<EdwardsProjective, Size>;
    type TestCRHGadget = PedersenCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

    #[test]
    fn primitive_gadget_test() {
        primitive_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>(PEDERSEN_HASH_CONSTRAINTS)
    }

    #[test]
    fn masked_gadget_test() {
        masked_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>()
    }
}

mod pedersen_crh_gadget_on_affine {
    use super::*;

    type TestCRH = PedersenCRH<EdwardsAffine, Size>;
    type TestCRHGadget = PedersenCRHGadget<EdwardsAffine, Fr, EdwardsBlsGadget>;

    #[test]
    fn primitive_gadget_test() {
        primitive_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>(PEDERSEN_HASH_CONSTRAINTS_ON_AFFINE)
    }
}

mod pedersen_compressed_crh_gadget_on_projective {
    use super::*;

    type TestCRH = PedersenCompressedCRH<EdwardsProjective, Size>;
    type TestCRHGadget = PedersenCompressedCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

    #[test]
    fn primitive_gadget_test() {
        primitive_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>(PEDERSEN_HASH_CONSTRAINTS)
    }

    #[test]
    fn masked_gadget_test() {
        masked_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>()
    }
}

// Note: Bowe-Hopwood CRH Gadget currently does not support affine curves or masked crh

mod bowe_hopwood_pedersen_crh_gadget_on_projective {
    use super::*;

    type TestCRH = BoweHopwoodPedersenCRH<EdwardsProjective, BoweHopwoodSize>;
    type TestCRHGadget = BoweHopwoodPedersenCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

    #[test]
    fn primitive_gadget_test() {
        primitive_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>(BOWE_HOPWOOD_HASH_CONSTRAINTS)
    }
}

mod bowe_hopwood_pedersen_compressed_crh_gadget_on_projective {
    use super::*;

    type TestCRH = BoweHopwoodPedersenCompressedCRH<EdwardsProjective, BoweHopwoodSize>;
    type TestCRHGadget = BoweHopwoodPedersenCompressedCRHGadget<EdwardsProjective, Fr, EdwardsBlsGadget>;

    #[test]
    fn primitive_gadget_test() {
        primitive_crh_gadget_test::<Fr, TestCRH, TestCRHGadget>(BOWE_HOPWOOD_HASH_CONSTRAINTS)
    }
}
