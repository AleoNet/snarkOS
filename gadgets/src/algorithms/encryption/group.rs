use snarkos_algorithms::encryption::GroupEncryption;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, ProjectiveCurve},
    gadgets::{curves::GroupGadget, r1cs::ConstraintSystem, utilities::alloc::AllocGadget},
};

use std::{borrow::Borrow, marker::PhantomData};

pub struct GroupEncryptionParametersGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    parameters: GG,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field + ProjectiveCurve, GG: GroupGadget<G, F>> AllocGadget<GroupEncryption<G>, F>
    for GroupEncryptionParametersGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<GroupEncryption<G>>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            parameters: GG::alloc_checked(cs, || f().map(|pp| pp.borrow().parameters))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<GroupEncryption<G>>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            parameters: GG::alloc_input(cs, || f().map(|pp| pp.borrow().parameters))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> Clone for GroupEncryptionParametersGadget<G, F, GG> {
    fn clone(&self) -> Self {
        Self {
            parameters: self.parameters.clone(),
            _group: PhantomData,
            _engine: PhantomData,
        }
    }
}
