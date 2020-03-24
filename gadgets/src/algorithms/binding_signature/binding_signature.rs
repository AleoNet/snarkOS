use crate::algorithms::commitment::pedersen::*;

use snarkos_algorithms::{commitment::PedersenCommitment, crh::PedersenSize};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField},
    gadgets::{
        algorithms::{BindingSignatureGadget, CommitmentGadget},
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, uint8::UInt8},
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
        parameters: &Self::ParametersGadget,
        partial_bvk: &Self::OutputGadget,
        value_balance: u64,
        c: &Self::RandomnessGadget,
        affine_r: &Self::OutputGadget,
        recommit: &Self::OutputGadget,
    ) -> Result<(), SynthesisError> {
        let value_balance_bytes = UInt8::alloc_vec(cs.ns(|| "value_balance_bytes"), &value_balance.to_le_bytes())?;

        let value_balance_comm = Self::check_value_balance_commitment_gadget(
            &mut cs.ns(|| "value_balance_commitment"),
            &parameters,
            &value_balance_bytes,
        )?;

        let bvk = partial_bvk.sub(cs.ns(|| "construct_bvk"), &value_balance_comm)?;

        let c_bits: Vec<_> = c.0.iter().flat_map(|byte| byte.into_bits_le()).collect();
        let zero = GG::zero(&mut cs.ns(|| "zero")).unwrap();

        let result = bvk.mul_bits(cs.ns(|| "mul_bits"), &zero, c_bits.iter())?;

        let result = result
            .add(cs.ns(|| "add_affine_r"), &affine_r)?
            .sub(cs.ns(|| "sub_recommit"), &recommit)?;

        result.enforce_equal(&mut cs.ns(|| "Check that the binding signature verifies"), &zero)?;

        Ok(())
    }
}
