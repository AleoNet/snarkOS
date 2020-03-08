use crate::{
    algorithms::CommitmentScheme,
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait CommitmentGadget<C: CommitmentScheme, F: Field> {
    type OutputGadget: EqGadget<F> + ToBytesGadget<F> + AllocGadget<C::Output, F> + Clone + Sized + Debug;
    type ParametersGadget: AllocGadget<C::Parameters, F> + Clone;
    type RandomnessGadget: AllocGadget<C::Randomness, F> + Clone;

    fn check_commitment_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        parameters: &Self::ParametersGadget,
        input: &[UInt8],
        r: &Self::RandomnessGadget,
    ) -> Result<Self::OutputGadget, SynthesisError>;
}

// pub trait CompressedCommitmentGadget<C: CommitmentScheme, F: Field> {
//     type OutputGadget: EqGadget<F>
//     + ToBytesGadget<F>
//     + AllocGadget<C::Output, F>
//     + Clone
//     + Sized
//     + Debug;
//     type ParametersGadget: AllocGadget<C::Parameters, F> + Clone;
//     type RandomnessGadget: AllocGadget<C::Randomness, F> + Clone;
//
//     fn check_compressed_commitment_gadget<CS: ConstraintSystem<F>>(
//         cs: CS,
//         parameters: &Self::ParametersGadget,
//         input: &[UInt8],
//         r: &Self::RandomnessGadget,
//     ) -> Result<Self::OutputGadget, SynthesisError>;
// }
