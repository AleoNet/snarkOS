// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use snarkos_algorithms::signature::{SchnorrParameters, SchnorrPublicKey, SchnorrSignature};
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
            uint::unsigned_integer::{UInt, UInt8},
            ToBytesGadget,
        },
    },
};
use snarkos_utilities::serialize::{CanonicalDeserialize, CanonicalSerialize};

use digest::Digest;
use itertools::Itertools;
use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone)]
pub struct SchnorrParametersGadget<G: Group, F: Field, D: Digest> {
    parameters: SchnorrParameters<G, D>,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group, F: Field, D: Digest> AllocGadget<SchnorrParameters<G, D>, F> for SchnorrParametersGadget<G, F, D> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<SchnorrParameters<G, D>>, CS: ConstraintSystem<F>>(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let value = value_gen()?;
        let parameters = value.borrow().clone();
        Ok(Self {
            parameters,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<SchnorrParameters<G, D>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let value = value_gen()?;
        let parameters = value.borrow().clone();
        Ok(Self {
            parameters,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchnorrPublicKeyGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    public_key: GG,
    _group: PhantomData<G>,
    _engine: PhantomData<F>,
}

impl<G: Group + CanonicalSerialize + CanonicalDeserialize, F: Field, GG: GroupGadget<G, F>>
    AllocGadget<SchnorrPublicKey<G>, F> for SchnorrPublicKeyGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<SchnorrPublicKey<G>>, CS: ConstraintSystem<F>>(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_checked(cs, || f().map(|pk| pk.borrow().0))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<SchnorrPublicKey<G>>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_input(cs, || f().map(|pk| pk.borrow().0))?,
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

impl<G: Group + CanonicalSerialize + CanonicalDeserialize, GG: GroupGadget<G, F>, D: Digest + Send + Sync, F: Field>
    SignaturePublicKeyRandomizationGadget<SchnorrSignature<G, D>, F> for SchnorrPublicKeyRandomizationGadget<G, F, GG>
{
    type ParametersGadget = SchnorrParametersGadget<G, F, D>;
    type PublicKeyGadget = SchnorrPublicKeyGadget<G, F, GG>;

    fn check_randomization_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        public_key: &Self::PublicKeyGadget,
        randomness: &[UInt8],
    ) -> Result<Self::PublicKeyGadget, SynthesisError> {
        let randomness = randomness.iter().flat_map(|b| b.to_bits_le()).collect::<Vec<_>>();
        let mut rand_pk = public_key.public_key.clone();
        rand_pk.precomputed_base_scalar_mul(
            cs.ns(|| "check_randomization_gadget"),
            randomness.iter().zip_eq(&parameters.parameters.generator_powers),
        )?;

        Ok(SchnorrPublicKeyGadget {
            public_key: rand_pk,
            _group: PhantomData,
            _engine: PhantomData,
        })
    }
}
