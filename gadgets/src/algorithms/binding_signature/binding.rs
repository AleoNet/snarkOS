use crate::algorithms::commitment::pedersen::*;

use snarkos_algorithms::{commitment::PedersenCommitment, crh::PedersenSize};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField, ProjectiveCurve},
    gadgets::{
        algorithms::{BindingSignatureGadget, CommitmentGadget},
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::uint8::UInt8,
    },
};
//use snarkos_utilities::{bytes::ToBytes, to_bytes};

//use std::{borrow::Borrow, marker::PhantomData};
use std::marker::PhantomData;

pub struct BindingSignatureVerificationGadget<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>>(
    PhantomData<G>,
    PhantomData<GG>,
    PhantomData<F>,
);

impl<F: PrimeField, G: Group + ProjectiveCurve, GG: GroupGadget<G, F>, S: PedersenSize>
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
}
