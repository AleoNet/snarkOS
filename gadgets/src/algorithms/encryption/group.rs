use snarkos_algorithms::encryption::GroupEncryption;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, Group, PrimeField, ProjectiveCurve},
    gadgets::{
        algorithms::EncryptionGadget,
        curves::{CompressedGroupGadget, GroupGadget},
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget},
            uint::UInt8,
        },
    },
};
use snarkos_utilities::{to_bytes, ToBytes};

use std::{borrow::Borrow, marker::PhantomData};

// Parameters
#[derive(Debug, PartialEq, Eq)]
pub struct GroupEncryptionParametersGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    parameters: GG,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> AllocGadget<G, F>
    for GroupEncryptionParametersGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            parameters: GG::alloc_checked(cs, || f().map(|pp| pp))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            parameters: GG::alloc_input(cs, || f().map(|pp| pp))?,
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

// GroupEncryption Private Key Gadget
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionPrivateKeyGadget<G: Group>(pub Vec<UInt8>, PhantomData<G>);

impl<G: Group, F: PrimeField> AllocGadget<G::ScalarField, F> for GroupEncryptionPrivateKeyGadget<G> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G::ScalarField>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let private_key = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(GroupEncryptionPrivateKeyGadget(
            UInt8::alloc_vec(cs, &private_key)?,
            PhantomData,
        ))
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G::ScalarField>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let private_key = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(GroupEncryptionPrivateKeyGadget(
            UInt8::alloc_vec(cs, &private_key)?,
            PhantomData,
        ))
    }
}

// GroupEncryption Public Key Gadget
#[derive(Debug, PartialEq, Eq)]
pub struct GroupEncryptionPublicKeyGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    public_key: GG,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> AllocGadget<G, F>
    for GroupEncryptionPublicKeyGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_checked(cs, || f().map(|pp| pp))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_input(cs, || f().map(|pp| pp))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> Clone for GroupEncryptionPublicKeyGadget<G, F, GG> {
    fn clone(&self) -> Self {
        Self {
            public_key: self.public_key.clone(),
            _group: PhantomData,
            _engine: PhantomData,
        }
    }
}

// GroupEncryption Plaintext Gadget
#[derive(Debug, PartialEq, Eq)]
pub struct GroupEncryptionPlaintextGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    plaintext: Vec<GG>,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> AllocGadget<Vec<G>, F>
    for GroupEncryptionPlaintextGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = f().map(|pp| pp.borrow().clone())?;

        let mut plaintext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc_checked(cs.ns(|| format!("Iteration {}", i)), || Ok(value.borrow()))?;
            plaintext.push(alloc_group);
        }

        Ok(Self {
            plaintext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = f().map(|pp| pp.borrow().clone())?;

        let mut plaintext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc_input(cs.ns(|| format!("Iteration {}", i)), || Ok(value.borrow()))?;
            plaintext.push(alloc_group);
        }

        Ok(Self {
            plaintext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> Clone for GroupEncryptionPlaintextGadget<G, F, GG> {
    fn clone(&self) -> Self {
        Self {
            plaintext: self.plaintext.clone(),
            _group: PhantomData,
            _engine: PhantomData,
        }
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> ConditionalEqGadget<F> for GroupEncryptionPlaintextGadget<G, F, GG> {
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (m_i, other_m_i)) in self.plaintext.iter().zip(&other.plaintext).enumerate() {
            m_i.conditional_enforce_equal(
                &mut cs.ns(|| format!("conditional_enforce_equal index {}", i)),
                &other_m_i,
                condition,
            )?;
        }

        Ok(())
    }

    fn cost() -> usize {
        unimplemented!()
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> EqGadget<F> for GroupEncryptionPlaintextGadget<G, F, GG> {}

// GroupEncryption Ciphertext Gadget
#[derive(Debug, PartialEq, Eq)]
pub struct GroupEncryptionCiphertextGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    plaintext: Vec<GG>,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> AllocGadget<Vec<G>, F>
    for GroupEncryptionCiphertextGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = f().map(|pp| pp.borrow().clone())?;

        let mut plaintext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc_checked(cs.ns(|| format!("Iteration {}", i)), || Ok(value.borrow()))?;
            plaintext.push(alloc_group);
        }

        Ok(Self {
            plaintext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = f().map(|pp| pp.borrow().clone())?;

        let mut plaintext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc_input(cs.ns(|| format!("Iteration {}", i)), || Ok(value.borrow()))?;
            plaintext.push(alloc_group);
        }

        Ok(Self {
            plaintext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> Clone for GroupEncryptionCiphertextGadget<G, F, GG> {
    fn clone(&self) -> Self {
        Self {
            plaintext: self.plaintext.clone(),
            _group: PhantomData,
            _engine: PhantomData,
        }
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> ConditionalEqGadget<F> for GroupEncryptionCiphertextGadget<G, F, GG> {
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (m_i, other_m_i)) in self.plaintext.iter().zip(&other.plaintext).enumerate() {
            m_i.conditional_enforce_equal(
                &mut cs.ns(|| format!("conditional_enforce_equal index {}", i)),
                &other_m_i,
                condition,
            )?;
        }

        Ok(())
    }

    fn cost() -> usize {
        unimplemented!()
    }
}

impl<G: Group, F: Field, GG: GroupGadget<G, F>> EqGadget<F> for GroupEncryptionCiphertextGadget<G, F, GG> {}

// Group Encryption Gadget

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionGadget<G: Group + ProjectiveCurve, F: PrimeField, GG: CompressedGroupGadget<G, F>> {
    _group: PhantomData<fn() -> G>,
    _group_gadget: PhantomData<fn() -> GG>,
    _engine: PhantomData<F>,
}

impl<F: PrimeField, G: Group + ProjectiveCurve, GG: CompressedGroupGadget<G, F>> EncryptionGadget<GroupEncryption<G>, F>
    for GroupEncryptionGadget<G, F, GG>
{
    type CiphertextGadget = GroupEncryptionCiphertextGadget<G, F, GG>;
    type ParametersGadget = GroupEncryptionParametersGadget<G, F, GG>;
    type PlaintextGadget = GroupEncryptionPlaintextGadget<G, F, GG>;
    type PrivateKeyGadget = GroupEncryptionPrivateKeyGadget<G>;
    type PublicKeyGadget = GroupEncryptionPublicKeyGadget<G, F, GG>;

    //    fn check_encryption_gadget<CS: ConstraintSystem<F>>(
    //        cs: CS,
    //        parameters: &Self::ParametersGadget,
    //        public_key: &Self::PublicKeyGadget,
    //        input: &Self::PlaintextGadget,
    //    ) -> Result<Self::CiphertextGadget, SynthesisError> {
    //
    //    }

    //    fn check_decryption_gadget<CS: ConstraintSystem<F>>(
    //        cs: CS,
    //        parameters: &Self::ParametersGadget,
    //        private_key: &Self::PrivateKeyGadget,
    //        input: &Self::CiphertextGadget,
    //    ) -> Result<Self::PlaintextGadget, SynthesisError> {
    //
    //    }
}
