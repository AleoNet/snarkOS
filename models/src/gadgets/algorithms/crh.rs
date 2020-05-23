use crate::{
    curves::{Field, PrimeField},
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            eq::{ConditionalEqGadget, EqGadget},
            select::CondSelectGadget,
            uint8::UInt8,
            ToBytesGadget,
        },
    },
};
use snarkvm_errors::gadgets::SynthesisError;
use snarkvm_models::algorithms::CRH;

use std::fmt::Debug;

pub trait CRHGadget<H: CRH, F: Field>: Sized {
    type ParametersGadget: AllocGadget<H::Parameters, F> + Clone;
    type OutputGadget: ConditionalEqGadget<F>
        + EqGadget<F>
        + ToBytesGadget<F>
        + CondSelectGadget<F>
        + AllocGadget<H::Output, F>
        + Debug
        + Clone
        + Sized;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}

pub trait MaskedCRHGadget<H: CRH, F: PrimeField>: CRHGadget<H, F> {
    /// Extends the mask such that 0 => 01, 1 => 10.
    fn extend_mask<CS: ConstraintSystem<F>>(_: CS, mask: &[UInt8]) -> Result<Vec<UInt8>, SynthesisError> {
        let extended_mask = mask
            .iter()
            .flat_map(|m| {
                m.into_bits_le()
                    .chunks(4)
                    .map(|c| {
                        let new_byte = c.into_iter().flat_map(|b| vec![*b, b.not()]).collect::<Vec<_>>();
                        UInt8::from_bits_le(&new_byte)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(extended_mask)
    }

    fn check_evaluation_gadget_masked<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        mask: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}
