use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{MontgomeryModelParameters, PrimeField},
    gadgets::{curves::FpGadget, r1cs::ConstraintSystem, utilities::alloc::AllocGadget},
};
// use snarkos_utilities::{to_bytes, ToBytes};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone, Debug)]
pub struct Elligator2FieldGadget<P: MontgomeryModelParameters, F: PrimeField>(pub FpGadget<F>, PhantomData<P>);

impl<P: MontgomeryModelParameters, F: PrimeField> AllocGadget<[u8], F> for Elligator2FieldGadget<P, F> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[u8]>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Elligator2FieldGadget(
            FpGadget::alloc(cs, || match value_gen() {
                Ok(value) => Ok(F::read(&value.borrow()[..])?),
                Err(_) => Err(SynthesisError::AssignmentMissing),
            })?,
            PhantomData,
        ))
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<[u8]>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Elligator2FieldGadget(
            FpGadget::alloc_input(cs, || match value_gen() {
                Ok(value) => Ok(F::read(&value.borrow()[..])?),
                Err(_) => Err(SynthesisError::AssignmentMissing),
            })?,
            PhantomData,
        ))
    }
}
