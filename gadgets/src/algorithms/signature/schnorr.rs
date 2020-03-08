use snarkos_algorithms::signature::{SchnorrParameters, SchnorrSignature};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group},
    gadgets::{
        algorithms::SignaturePublicKeyRandomizationGadget,
        curves::GroupGadget,
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget},
            uint8::UInt8,
            ToBytesGadget,
        },
    },
};

use digest::Digest;
use std::{borrow::Borrow, marker::PhantomData};

pub struct SchnorrParametersGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    generator: GG,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>, D: Digest> AllocGadget<SchnorrParameters<G, D>, F>
    for SchnorrParametersGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<SchnorrParameters<G, D>>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            generator: GG::alloc_checked(cs, || f().map(|pp| pp.borrow().generator))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<SchnorrParameters<G, D>>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            generator: GG::alloc_input(cs, || f().map(|pp| pp.borrow().generator))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> Clone for SchnorrParametersGadget<G, F, GG> {
    fn clone(&self) -> Self {
        Self {
            generator: self.generator.clone(),
            _group: PhantomData,
            _engine: PhantomData,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchnorrPublicKeyGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    public_key: GG,
    _group: PhantomData<G>,
    _engine: PhantomData<F>,
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> AllocGadget<G, F> for SchnorrPublicKeyGadget<G, F, GG> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_input(cs, || f().map(|pk| *pk.borrow()))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_input(cs, || f().map(|pk| *pk.borrow()))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> ConditionalEqGadget<F> for SchnorrPublicKeyGadget<G, F, GG> {
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        self.public_key.conditional_enforce_equal(
            &mut cs.ns(|| "conditional_enforce_equal"),
            &other.public_key,
            condition,
        )?;
        Ok(())
    }

    fn cost() -> usize {
        <GG as ConditionalEqGadget<F>>::cost()
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> EqGadget<F> for SchnorrPublicKeyGadget<G, F, GG> {}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> ToBytesGadget<F> for SchnorrPublicKeyGadget<G, F, GG> {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.public_key.to_bytes(&mut cs.ns(|| "to_bytes"))
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.public_key.to_bytes_strict(&mut cs.ns(|| "to_bytes_strict"))
    }
}

pub struct SchnorrPublicKeyRandomizationGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    _group: PhantomData<*const G>,
    _group_gadget: PhantomData<*const GG>,
    _engine: PhantomData<*const F>,
}

impl<G: Group, GG: GroupGadget<G, F>, D: Digest + Send + Sync, F: Field>
    SignaturePublicKeyRandomizationGadget<SchnorrSignature<G, D>, F> for SchnorrPublicKeyRandomizationGadget<G, F, GG>
{
    type ParametersGadget = SchnorrParametersGadget<G, F, GG>;
    type PublicKeyGadget = SchnorrPublicKeyGadget<G, F, GG>;

    fn check_randomization_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        public_key: &Self::PublicKeyGadget,
        randomness: &[UInt8],
    ) -> Result<Self::PublicKeyGadget, SynthesisError> {
        let base = parameters.generator.clone();
        let randomness = randomness.iter().flat_map(|b| b.into_bits_le()).collect::<Vec<_>>();
        let rand_pk = base.mul_bits(
            &mut cs.ns(|| "check_randomization_gadget"),
            &public_key.public_key,
            randomness.iter(),
        )?;
        Ok(SchnorrPublicKeyGadget {
            public_key: rand_pk,
            _group: PhantomData,
            _engine: PhantomData,
        })
    }
}
