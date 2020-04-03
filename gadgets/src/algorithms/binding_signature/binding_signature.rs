use crate::algorithms::commitment::pedersen::*;

use snarkos_algorithms::{commitment::PedersenCompressedCommitment, crh::PedersenSize};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField, ProjectiveCurve},
    gadgets::{
        algorithms::{BindingSignatureGadget, CommitmentGadget},
        curves::CompressedGroupGadget,
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, boolean::Boolean, uint8::UInt8},
    },
};

use std::marker::PhantomData;

pub struct BindingSignatureVerificationGadget<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>>(
    PhantomData<G>,
    PhantomData<GG>,
    PhantomData<F>,
);

impl<F: PrimeField, G: Group + ProjectiveCurve, GG: CompressedGroupGadget<G, F>, S: PedersenSize>
    BindingSignatureGadget<PedersenCompressedCommitment<G, S>, F, G> for BindingSignatureVerificationGadget<G, F, GG>
{
    type CompressedOutputGadget = GG::BaseFieldGadget;
    type OutputGadget = GG;
    type ParametersGadget = PedersenCommitmentParametersGadget<G, S, F>;
    type RandomnessGadget = PedersenRandomnessGadget<G>;

    fn check_value_balance_commitment_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let default_randomness = Self::RandomnessGadget::alloc(&mut cs.ns(|| "default_randomness"), || {
            Ok(<G as Group>::ScalarField::default())
        })?;

        let output =
            PedersenCommitmentGadget::<G, F, GG>::check_commitment_gadget(cs, parameters, input, &default_randomness)?;
        Ok(output)
    }

    fn check_binding_signature_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        partial_bvk: &Self::OutputGadget,
        value_balance_comm: &Self::OutputGadget,
        is_negative: &Boolean,
        c: &Self::RandomnessGadget,
        affine_r: &Self::OutputGadget,
        recommit: &Self::OutputGadget,
    ) -> Result<(), SynthesisError> {
        // TODO make this circuit more efficient

        let negative_bvk = partial_bvk.add(cs.ns(|| "construct_negative_bvk"), &value_balance_comm)?;
        let positive_bvk = partial_bvk.sub(cs.ns(|| "construct_negative_bvk"), &value_balance_comm)?;

        let c_bits: Vec<_> = c.0.iter().flat_map(|byte| byte.into_bits_le()).collect();
        let zero = GG::zero(&mut cs.ns(|| "zero")).unwrap();

        let negative_result = negative_bvk.mul_bits(cs.ns(|| "mul_bits_negative"), &zero, c_bits.iter())?;
        let positive_result = positive_bvk.mul_bits(cs.ns(|| "mul_bits_positive"), &zero, c_bits.iter())?;

        let temp = affine_r.sub(cs.ns(|| "sub_recommit"), &recommit)?;
        let negative_result = negative_result.add(cs.ns(|| "add_temp"), &temp)?;
        let positive_result = positive_result.add(cs.ns(|| "add_temp2"), &temp)?;

        let result = GG::conditionally_select(
            cs.ns(|| "select result"),
            &is_negative.not(),
            &positive_result,
            &negative_result,
        )?;

        result.enforce_equal(&mut cs.ns(|| "Check that the binding signature verifies"), &zero)?;

        Ok(())
    }
}
