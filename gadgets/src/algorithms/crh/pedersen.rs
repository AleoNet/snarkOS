use snarkos_algorithms::crh::{PedersenCRH, PedersenCRHParameters, PedersenCompressedCRH, PedersenSize};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField, ProjectiveCurve},
    gadgets::{
        algorithms::{CRHGadget, MaskedCRHGadget},
        curves::{CompressedGroupGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, boolean::Boolean, uint8::UInt8},
    },
};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);
        // Pad the input if it is not the correct length.
        let input_in_bits = pad_input_and_bitify::<S>(input);

        Ok(GG::precomputed_base_multiscalar_mul(
            cs,
            &parameters.parameters.bases,
            input_in_bits.chunks(S::WINDOW_SIZE),
        )?)
    }
}

fn pad_input_and_bitify<S: PedersenSize>(input: &[UInt8]) -> Vec<Boolean> {
    let mut padded_input = input.to_vec();
    padded_input.resize(S::WINDOW_SIZE * S::NUM_WINDOWS / 8, UInt8::constant(0u8));
    assert_eq!(padded_input.len() * 8, S::WINDOW_SIZE * S::NUM_WINDOWS);
    padded_input.into_iter().flat_map(|byte| byte.into_bits_le()).collect()
}

impl<F: PrimeField, G: Group, GG: GroupGadget<G, F>, S: PedersenSize> MaskedCRHGadget<PedersenCRH<G, S>, F>
    for PedersenCRHGadget<G, F, GG>
{
    /// Evaluates a masked Pedersen hash on the given `input` using the given `mask`. The algorithm
    /// is based on the description in https://eprint.iacr.org/2020/190.pdf, which relies on the
    /// homomorphic properties of Pedersen hashes. First, the mask is extended to ensure constant
    /// hardness - for each bit, 0 => 01, 1 => 10. Then, denoting input bits as m_i, mask bits
    /// as p_i and bases as h_i, computes sum of
    /// (g_i * 1[p_i = 0] + g_i^{-1} * 1[p_i = 1])^{m_i \xor p_i} for all i. Finally, the hash of
    /// the mask itself, being sum of h_i^{p_i} for all i, is added to the computed sum. This
    /// algorithm ensures that each bit in the hash is affected by the mask and that the
    /// final hash remains the same as if no mask was used.
    fn check_evaluation_gadget_masked<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        mask: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        // The mask will be extended to ensure constant hardness. This condition
        // ensures the input and the mask sizes match.
        if input.len() != mask.len() * 2 {
            return Err(SynthesisError::Unsatisfiable);
        }
        let mask = <Self as MaskedCRHGadget<PedersenCRH<G, S>, F>>::extend_mask(cs.ns(|| "extend mask"), mask)?;
        // H(p) = sum of h_i^{p_i} for all i.
        let mask_hash = Self::check_evaluation_gadget(cs.ns(|| "evaluate mask"), parameters, &mask)?;

        assert_eq!(parameters.parameters.bases.len(), S::NUM_WINDOWS);
        // Pad the input if it is not the correct length.
        let input_in_bits = pad_input_and_bitify::<S>(input);
        let mask_in_bits = pad_input_and_bitify::<S>(&mask);

        let masked_output = GG::precomputed_base_multiscalar_mul_masked(
            cs.ns(|| "multiscalar multiplication"),
            &parameters.parameters.bases,
            input_in_bits.chunks(S::WINDOW_SIZE),
            mask_in_bits.chunks(S::WINDOW_SIZE),
        )?;
        masked_output.add(cs.ns(|| "remove mask"), &mask_hash)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PedersenCompressedCRHGadget<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> {
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<F>,
}

impl<F: Field, G: Group + ProjectiveCurve, GG: CompressedGroupGadget<G, F>, S: PedersenSize>
    CRHGadget<PedersenCompressedCRH<G, S>, F> for PedersenCompressedCRHGadget<G, F, GG>
{
    type OutputGadget = GG::BaseFieldGadget;
    type ParametersGadget = PedersenCRHParametersGadget<G, S, F, GG>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let output = PedersenCRHGadget::<G, F, GG>::check_evaluation_gadget(cs, parameters, input)?;
        Ok(output.to_x_coordinate())
    }
}

impl<F: PrimeField, G: Group + ProjectiveCurve, GG: CompressedGroupGadget<G, F>, S: PedersenSize>
    MaskedCRHGadget<PedersenCompressedCRH<G, S>, F> for PedersenCompressedCRHGadget<G, F, GG>
{
    fn check_evaluation_gadget_masked<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        mask: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let output = PedersenCRHGadget::<G, F, GG>::check_evaluation_gadget_masked(cs, parameters, input, mask)?;
        Ok(output.to_x_coordinate())
    }
}
