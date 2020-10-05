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

use snarkos_algorithms::encryption::{GroupEncryption, GroupEncryptionParameters, GroupEncryptionPublicKey};
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
            uint::{UInt, UInt8},
            ToBytesGadget,
        },
    },
};
use snarkos_utilities::{to_bytes, ToBytes};

use itertools::Itertools;
use std::{borrow::Borrow, marker::PhantomData};

/// Group encryption parameters gadget
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionParametersGadget<G: Group> {
    parameters: GroupEncryptionParameters<G>,
}

impl<G: Group + ProjectiveCurve, F: Field> AllocGadget<GroupEncryptionParameters<G>, F>
    for GroupEncryptionParametersGadget<G>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<GroupEncryptionParameters<G>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let value = value_gen()?;
        let parameters = value.borrow().clone();
        Ok(Self { parameters })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<GroupEncryptionParameters<G>>,
        CS: ConstraintSystem<F>,
    >(
        _cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let value = value_gen()?;
        let parameters = value.borrow().clone();
        Ok(Self { parameters })
    }
}

/// Group encryption private key gadget
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

impl<G: Group, F: PrimeField> ToBytesGadget<F> for GroupEncryptionPrivateKeyGadget<G> {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.0.to_bytes(&mut cs.ns(|| "to_bytes"))
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.0.to_bytes_strict(&mut cs.ns(|| "to_bytes_strict"))
    }
}

/// Group encryption randomness gadget
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionRandomnessGadget<G: Group>(pub Vec<UInt8>, PhantomData<G>);

impl<G: Group, F: PrimeField> AllocGadget<G::ScalarField, F> for GroupEncryptionRandomnessGadget<G> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G::ScalarField>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let randomness = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(GroupEncryptionRandomnessGadget(
            UInt8::alloc_vec(cs, &randomness)?,
            PhantomData,
        ))
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<G::ScalarField>, CS: ConstraintSystem<F>>(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let randomness = to_bytes![value_gen()?.borrow()].unwrap();
        Ok(GroupEncryptionRandomnessGadget(
            UInt8::alloc_vec(cs, &randomness)?,
            PhantomData,
        ))
    }
}

/// Group encryption blinding exponents gadget
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionBlindingExponentsGadget<G: Group>(pub Vec<Vec<UInt8>>, PhantomData<G>);

impl<G: Group, F: PrimeField> AllocGadget<Vec<G::ScalarField>, F> for GroupEncryptionBlindingExponentsGadget<G> {
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G::ScalarField>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = value_gen().map(|pp| pp.borrow().clone())?;

        let mut blinding_exponents = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc = UInt8::alloc_vec(cs.ns(|| format!("Blinding Exponent Iteration {}", i)), &to_bytes![
                value.borrow()
            ]?)?;
            blinding_exponents.push(alloc);
        }

        Ok(GroupEncryptionBlindingExponentsGadget(blinding_exponents, PhantomData))
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<Vec<G::ScalarField>>,
        CS: ConstraintSystem<F>,
    >(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = value_gen().map(|pp| pp.borrow().clone())?;

        let mut blinding_exponents = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc = UInt8::alloc_input_vec(cs.ns(|| format!("Blinding Exponent Iteration {}", i)), &to_bytes![
                value.borrow()
            ]?)?;
            blinding_exponents.push(alloc);
        }

        Ok(GroupEncryptionBlindingExponentsGadget(blinding_exponents, PhantomData))
    }
}

/// Group encryption public key gadget
#[derive(Debug, PartialEq, Eq)]
pub struct GroupEncryptionPublicKeyGadget<G: Group, F: Field, GG: GroupGadget<G, F>> {
    public_key: GG,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> AllocGadget<GroupEncryptionPublicKey<G>, F>
    for GroupEncryptionPublicKeyGadget<G, F, GG>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<GroupEncryptionPublicKey<G>>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_checked(cs, || f().map(|pp| pp.borrow().0))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<GroupEncryptionPublicKey<G>>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Self {
            public_key: GG::alloc_input(cs, || f().map(|pp| pp.borrow().0))?,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> ToBytesGadget<F>
    for GroupEncryptionPublicKeyGadget<G, F, GG>
{
    /// Writes the x-coordinate of the encryption public key.
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.public_key.to_x_coordinate().to_bytes(&mut cs.ns(|| "to_bytes"))
    }

    /// Writes the x-coordinate of the encryption public key. Additionally checks if the
    /// generated list of booleans is 'valid'.
    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.public_key
            .to_x_coordinate()
            .to_bytes_strict(&mut cs.ns(|| "to_bytes_strict"))
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

impl<G: Group, F: Field, GG: GroupGadget<G, F>> ConditionalEqGadget<F> for GroupEncryptionPublicKeyGadget<G, F, GG> {
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

impl<G: Group, F: Field, GG: GroupGadget<G, F>> EqGadget<F> for GroupEncryptionPublicKeyGadget<G, F, GG> {}

/// Group encryption plaintext gadget
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
            let alloc_group = GG::alloc(cs.ns(|| format!("Plaintext Iteration {}", i)), || Ok(value.borrow()))?;
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
            let alloc_group = GG::alloc_input(cs.ns(|| format!("Plaintext Iteration {}", i)), || Ok(value.borrow()))?;
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

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> EqGadget<F>
    for GroupEncryptionPlaintextGadget<G, F, GG>
{
}

/// Group encryption ciphertext gadget
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionCiphertextGadget<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> {
    ciphertext: Vec<GG>,
    _group: PhantomData<*const G>,
    _engine: PhantomData<*const F>,
}

impl<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> AllocGadget<Vec<G>, F>
    for GroupEncryptionCiphertextGadget<G, F, GG>
{
    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = f().map(|pp| pp.borrow().clone())?;

        let mut ciphertext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc(cs.ns(|| format!("Ciphertext Iteration {}", i)), || Ok(value.borrow()))?;
            ciphertext.push(alloc_group);
        }

        Ok(Self {
            ciphertext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<Vec<G>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        f: Fn,
    ) -> Result<Self, SynthesisError> {
        let values = f().map(|pp| pp.borrow().clone())?;

        let mut ciphertext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc_input(cs.ns(|| format!("Ciphertext Iteration {}", i)), || Ok(value.borrow()))?;
            ciphertext.push(alloc_group);
        }

        Ok(Self {
            ciphertext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> ToBytesGadget<F>
    for GroupEncryptionCiphertextGadget<G, F, GG>
{
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut output_bytes = vec![];
        for (i, group_gadget) in self.ciphertext.iter().enumerate() {
            let group_bytes = group_gadget
                .to_x_coordinate()
                .to_bytes(&mut cs.ns(|| format!("to_bytes {}", i)))?;
            output_bytes.extend(group_bytes);
        }

        Ok(output_bytes)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut output_bytes = vec![];
        for (i, group_gadget) in self.ciphertext.iter().enumerate() {
            let group_bytes = group_gadget
                .to_x_coordinate()
                .to_bytes_strict(&mut cs.ns(|| format!("to_bytes_strict {}", i)))?;
            output_bytes.extend(group_bytes);
        }

        Ok(output_bytes)
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> ConditionalEqGadget<F>
    for GroupEncryptionCiphertextGadget<G, F, GG>
{
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (m_i, other_m_i)) in self.ciphertext.iter().zip(&other.ciphertext).enumerate() {
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

impl<G: Group + ProjectiveCurve, F: Field, GG: CompressedGroupGadget<G, F>> EqGadget<F>
    for GroupEncryptionCiphertextGadget<G, F, GG>
{
}

/// Group encryption gadget
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupEncryptionGadget<G: Group + ProjectiveCurve, F: PrimeField, GG: CompressedGroupGadget<G, F>> {
    _group: PhantomData<fn() -> G>,
    _group_gadget: PhantomData<fn() -> GG>,
    _engine: PhantomData<F>,
}

impl<G: Group + ProjectiveCurve, F: PrimeField, GG: CompressedGroupGadget<G, F>> EncryptionGadget<GroupEncryption<G>, F>
    for GroupEncryptionGadget<G, F, GG>
{
    type BlindingExponentGadget = GroupEncryptionBlindingExponentsGadget<G>;
    type CiphertextGadget = GroupEncryptionCiphertextGadget<G, F, GG>;
    type ParametersGadget = GroupEncryptionParametersGadget<G>;
    type PlaintextGadget = GroupEncryptionPlaintextGadget<G, F, GG>;
    type PrivateKeyGadget = GroupEncryptionPrivateKeyGadget<G>;
    type PublicKeyGadget = GroupEncryptionPublicKeyGadget<G, F, GG>;
    type RandomnessGadget = GroupEncryptionRandomnessGadget<G>;

    fn check_public_key_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        private_key: &Self::PrivateKeyGadget,
    ) -> Result<Self::PublicKeyGadget, SynthesisError> {
        let private_key_bits = private_key.0.iter().flat_map(|b| b.to_bits_le()).collect::<Vec<_>>();
        let mut public_key = GG::zero(&mut cs.ns(|| "zero"))?;
        public_key.precomputed_base_scalar_mul(
            cs.ns(|| "check_public_key_gadget"),
            private_key_bits.iter().zip_eq(&parameters.parameters.generator_powers),
        )?;

        Ok(GroupEncryptionPublicKeyGadget {
            public_key,
            _group: PhantomData,
            _engine: PhantomData,
        })
    }

    fn check_encryption_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,               // g
        randomness: &Self::RandomnessGadget,               // y
        public_key: &Self::PublicKeyGadget,                // record_owner
        input: &Self::PlaintextGadget,                     // m
        blinding_exponents: &Self::BlindingExponentGadget, // 1 [/] (z [+] j)
    ) -> Result<Self::CiphertextGadget, SynthesisError> {
        let zero = GG::zero(&mut cs.ns(|| "zero")).unwrap();

        let randomness_bits: Vec<_> = randomness.0.iter().flat_map(|byte| byte.clone().to_bits_le()).collect();

        let mut c_0 = zero.clone();
        c_0.precomputed_base_scalar_mul(
            cs.ns(|| "c_0"),
            randomness_bits.iter().zip_eq(&parameters.parameters.generator_powers),
        )?;

        let record_view_key_gadget =
            public_key
                .public_key
                .mul_bits(cs.ns(|| "record_view_key"), &zero, randomness_bits.into_iter())?;

        let z = record_view_key_gadget.to_x_coordinate();
        let z_bytes = z.to_bytes(&mut cs.ns(|| "z_to_bytes"))?;
        let z_bits: Vec<_> = z_bytes.into_iter().flat_map(|byte| byte.clone().to_bits_le()).collect();

        let mut ciphertext = vec![c_0];

        for (index, (blinding_exponent, m_j)) in blinding_exponents.0.iter().zip_eq(&input.plaintext).enumerate() {
            let j = index + 1;

            let cs = &mut cs.ns(|| format!("c_{}", j));

            let blinding_exponent_bits = blinding_exponent.iter().flat_map(|byte| byte.clone().to_bits_le());

            let h = record_view_key_gadget.mul_bits(cs.ns(|| "h"), &zero, blinding_exponent_bits)?;

            // z * h
            let h_z = h.mul_bits(cs.ns(|| "z * h"), &zero, z_bits.iter().copied())?;

            // j * h
            let h_j = {
                let mut internal_cs = cs.ns(|| format!("Construct {} * h_{}", j, j));

                let mut h_j = h.clone();

                let num_doubling = (j as f64).log2() as u32;
                for i in 0..num_doubling {
                    h_j.double_in_place(internal_cs.ns(|| format!("Double {}", i)))?;
                }

                let num_exponentiations = 2usize.pow(num_doubling);
                if j > num_exponentiations {
                    for i in 0..(j - num_exponentiations) {
                        h_j = h_j.add(internal_cs.ns(|| format!("Add: {}", i)), &h)?;
                    }
                }

                h_j
            };

            // (z_i [+] j) * h_i,j
            let expected_record_view_key = h_z.add(cs.ns(|| "expected record view key"), &h_j)?;

            expected_record_view_key.enforce_equal(
                &mut cs.ns(|| "Check that declared and computed record view keys are equal"),
                &record_view_key_gadget,
            )?;

            // Construct c_j
            let c_j = h.add(cs.ns(|| "construct c_j"), &m_j)?;

            ciphertext.push(c_j);
        }

        Ok(GroupEncryptionCiphertextGadget {
            ciphertext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}
