use crate::{
    algorithms::CRH,
    curves::Field,
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
use snarkos_errors::gadgets::SynthesisError;

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

pub trait MaskedCRHGadget<H: CRH, F: Field>: CRHGadget<H, F> {
    fn check_evaluation_gadget_masked<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        mask: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}
