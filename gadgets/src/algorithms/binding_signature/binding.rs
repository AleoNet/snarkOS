use crate::algorithms::commitment::pedersen::*;

use snarkos_algorithms::{commitment::PedersenCommitment, crh::PedersenSize};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField},
    gadgets::{
        algorithms::{BindingSignatureGadget, CommitmentGadget},
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::uint8::UInt8,
    },
};

use std::marker::PhantomData;

pub struct BindingSignatureVerificationGadget<G: Group, F: Field, GG: GroupGadget<G, F>>(
    PhantomData<G>,
    PhantomData<GG>,
    PhantomData<F>,
);

impl<F: PrimeField, G: Group, GG: GroupGadget<G, F>, S: PedersenSize>
    BindingSignatureGadget<PedersenCommitment<G, S>, F> for BindingSignatureVerificationGadget<G, F, GG>
{
    type OutputGadget = GG;
    type ParametersGadget = PedersenCommitmentParametersGadget<G, S, F>;
    type RandomnessGadget = PedersenRandomnessGadget<G>;

    fn check_commitment_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        randomness: &Self::RandomnessGadget,
    ) -> Result<Self::OutputGadget, SynthesisError> {
        let output = PedersenCommitmentGadget::<G, F, GG>::check_commitment_gadget(cs, parameters, input, randomness)?;
        Ok(output)
    }

    fn check_binding_signature_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        partial_bvk: &Self::OutputGadget,
        c: &Self::RandomnessGadget,
        affine_r: &Self::OutputGadget,
        recommit: &Self::OutputGadget,
    ) -> Result<bool, SynthesisError> {
        let c_bits: Vec<_> = c.0.iter().flat_map(|byte| byte.into_bits_le()).collect();
        let zero = GG::zero(&mut cs.ns(|| "zero")).unwrap();

        let result = zero.mul_bits(cs.ns(|| "mul_bits"), &partial_bvk, c_bits.iter())?;

        let result = result
            .add(cs.ns(|| "add_affine_r"), &affine_r)?
            .add(cs.ns(|| "add_recommit"), &recommit)?;

        result.enforce_equal(&mut cs.ns(|| "Check the binding signature verifies"), &zero)?;

        Ok(true)
    }
}
