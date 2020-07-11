use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Group, PrimeField, ProjectiveCurve},
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, uint::UInt8},
    },
};
use snarkos_utilities::{to_bytes, ToBytes};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone, Debug)]
pub struct Elligator2FieldGadget<G: Group>(pub Vec<UInt8>, PhantomData<G>);

impl<G: Group + ProjectiveCurve, F: PrimeField> AllocGadget<<G as ProjectiveCurve>::BaseField, F>
    for Elligator2FieldGadget<G>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<<G as ProjectiveCurve>::BaseField>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let element = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(Elligator2FieldGadget(UInt8::alloc_vec(cs, &element)?, PhantomData))
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<<G as ProjectiveCurve>::BaseField>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let element = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(Elligator2FieldGadget(
            UInt8::alloc_input_vec(cs, &element)?,
            PhantomData,
        ))
    }
}
