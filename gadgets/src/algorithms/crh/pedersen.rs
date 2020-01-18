use snarkos_algorithms::crh::{PedersenCRH, PedersenCRHParameters, PedersenSize};
use snarkos_models::{
    curves::{Field, Group},
    gadgets::{
        algorithms::CRHGadget,
        curves::GroupGadget,
        r1cs::{ConstraintSystem, SynthesisError},
        utilities::{alloc::AllocGadget, uint8::UInt8},
    },
};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone)]
pub struct PedersenCRHParametersGadget<G: Group, S: PedersenSize, F: Field, GG: GroupGadget<G, F>> {
    parameters: PedersenCRHParameters<G, S>,
    _group: PhantomData<GG>,
    _engine: PhantomData<F>,
}

impl<G: Group, S: PedersenSize, F: Field, GG: GroupGadget<G, F>> AllocGadget<PedersenCRHParameters<G, S>, F>
    for PedersenCRHParametersGadget<G, S, F, GG>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<PedersenCRHParameters<G, S>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(PedersenCRHParametersGadget {
            parameters: value_gen()?.borrow().clone(),
            _group: PhantomData,
            _engine: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<PedersenCRHParameters<G, S>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(PedersenCRHParametersGadget {
            parameters: value_gen()?.borrow().clone(),
            _group: PhantomData,
            _engine: PhantomData,
        })
    }
}

pub struct PedersenCRHGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<F>,
}

impl<F: Field, G: Group, GG: GroupGadget<G, F>, S: PedersenSize> CRHGadget<PedersenCRH<G, S>, F>
    for PedersenCRHGadget<G, F, GG>
{
    type OutputGadget = GG;
    type ParametersGadget = PedersenCRHParametersGadget<G, S, F, GG>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let mut padded_input = input.to_vec();
        // Pad the input if it is not the current length.
        if input.len() * 8 < S::WINDOW_SIZE * S::NUM_WINDOWS {
            let current_length = input.len();
            for _ in current_length..(S::WINDOW_SIZE * S::NUM_WINDOWS / 8) {
                padded_input.push(UInt8::constant(0u8));
            }
        }
        assert_eq!(padded_input.len() * 8, S::WINDOW_SIZE * S::NUM_WINDOWS);
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);

        // Allocate new variable for the result.
        let input_in_bits: Vec<_> = padded_input.iter().flat_map(|byte| byte.into_bits_le()).collect();
        let input_in_bits = input_in_bits.chunks(S::WINDOW_SIZE);

        Ok(GG::precomputed_base_multiscalar_mul(
            cs,
            &parameters.parameters.bases,
            input_in_bits,
        )?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::curves::edwards_bls12::EdwardsBlsGadget;
    use snarkos_curves::{bls12_377::Fr, edwards_bls12::EdwardsProjective};
    use snarkos_models::{
        algorithms::CRH,
        curves::ProjectiveCurve,
        gadgets::r1cs::{ConstraintSystem, TestConstraintSystem},
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
}
