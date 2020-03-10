use crate::{
    algorithms::PRF,
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, eq::EqGadget, uint8::UInt8, ToBytesGadget},
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait PRFGadget<P: PRF, F: Field> {
    type OutputGadget: EqGadget<F> + ToBytesGadget<F> + AllocGadget<P::Output, F> + Clone + Debug;

    fn new_seed<CS: ConstraintSystem<F>>(cs: CS, output: &P::Seed) -> Vec<UInt8>;

    fn check_evaluation_gadget<CS: ConstraintSystem<F>>(
        cs: CS,
        seed: &[UInt8],
        input: &[UInt8],
    ) -> Result<Self::OutputGadget, SynthesisError>;
}
