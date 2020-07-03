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
            uint::{UInt, UInt8},
            ToBytesGadget,
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

// GroupEncryption Randomness Gadget
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

// GroupEncryption Blinding Exponents Gadget
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
            let alloc = UInt8::alloc_vec(cs.ns(|| format!("Iteration {}", i)), &to_bytes![value.borrow()]?)?;
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
            let alloc = UInt8::alloc_input_vec(cs.ns(|| format!("Iteration {}", i)), &to_bytes![value.borrow()]?)?;
            blinding_exponents.push(alloc);
        }

        Ok(GroupEncryptionBlindingExponentsGadget(blinding_exponents, PhantomData))
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

impl<G: Group, F: Field, GG: GroupGadget<G, F>> ConditionalEqGadget<F> for GroupEncryptionPublicKeyGadget<G, F, GG> {
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        self.public_key.conditional_enforce_equal(
            &mut cs.ns(|| format!("conditional_enforce_equal")),
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
    ciphertext: Vec<GG>,
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

        let mut ciphertext = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let alloc_group = GG::alloc_checked(cs.ns(|| format!("Iteration {}", i)), || Ok(value.borrow()))?;
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
            let alloc_group = GG::alloc_input(cs.ns(|| format!("Iteration {}", i)), || Ok(value.borrow()))?;
            ciphertext.push(alloc_group);
        }

        Ok(Self {
            ciphertext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}

impl<G: Group + ProjectiveCurve, F: Field, GG: GroupGadget<G, F>> Clone for GroupEncryptionCiphertextGadget<G, F, GG> {
    fn clone(&self) -> Self {
        Self {
            ciphertext: self.ciphertext.clone(),
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

impl<G: Group, F: Field, GG: GroupGadget<G, F>> EqGadget<F> for GroupEncryptionCiphertextGadget<G, F, GG> {}

// Group Encryption Gadget

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
    type ParametersGadget = GroupEncryptionParametersGadget<G, F, GG>;
    type PlaintextGadget = GroupEncryptionPlaintextGadget<G, F, GG>;
    type PrivateKeyGadget = GroupEncryptionPrivateKeyGadget<G>;
    type PublicKeyGadget = GroupEncryptionPublicKeyGadget<G, F, GG>;
    type RandomnessGadget = GroupEncryptionRandomnessGadget<G>;

    fn check_public_key_gadget<CS: ConstraintSystem<F>>(
        mut cs: CS,
        parameters: &Self::ParametersGadget,
        private_key: &Self::PrivateKeyGadget,
    ) -> Result<Self::PublicKeyGadget, SynthesisError> {
        let base = parameters.parameters.clone();
        let zero = GG::zero(&mut cs.ns(|| "zero")).unwrap();

        let private_key_bits = private_key.0.iter().flat_map(|b| b.to_bits_le()).collect::<Vec<_>>();
        let public_key = base.mul_bits(&mut cs.ns(|| "check_public_key_gadget"), &zero, private_key_bits.iter())?;
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
        public_key: &Self::PublicKeyGadget,                // account_address
        input: &Self::PlaintextGadget,                     // m
        blinding_exponents: &Self::BlindingExponentGadget, // 1 [/] (z [+] j)
    ) -> Result<Self::CiphertextGadget, SynthesisError> {
        let zero = GG::zero(&mut cs.ns(|| "zero")).unwrap();

        let randomness_bits: Vec<_> = randomness.0.iter().flat_map(|byte| byte.clone().to_bits_le()).collect();
        let c_0 = parameters
            .parameters
            .mul_bits(cs.ns(|| "c_0"), &zero, randomness_bits.iter())?;

        let record_view_key_gadget =
            public_key
                .public_key
                .mul_bits(cs.ns(|| "shared_key"), &zero, randomness_bits.iter())?;

        let z = record_view_key_gadget.to_x_coordinate();
        let z_bytes = z.to_bytes(&mut cs.ns(|| "z_to_bytes"))?;
        let z_bits: Vec<_> = z_bytes.iter().flat_map(|byte| byte.clone().to_bits_le()).collect();

        let mut ciphertext = vec![c_0];
        for (index, (blinding_exponent, m_j)) in blinding_exponents.0.iter().zip(&input.plaintext).enumerate() {
            let j = index + 1;

            let cs = &mut cs.ns(|| format!("c_{}", j));

            let blinding_exponent_bits: Vec<_> = blinding_exponent
                .iter()
                .flat_map(|byte| byte.clone().to_bits_le())
                .collect();

            let h_j = record_view_key_gadget.mul_bits(cs.ns(|| "h_j"), &zero, blinding_exponent_bits.iter())?;

            // z * h_j
            let zh_j = h_j.mul_bits(cs.ns(|| "z * h_j"), &zero, z_bits.iter())?;

            // j * h_j
            let jh_j = {
                let mut jh_j_cs = cs.ns(|| format!("Construct {} * h_{}", j, j));

                let mut jh_j = h_j.clone();

                let num_doubling = (j as f64).log2() as u32;

                for i in 0..num_doubling {
                    jh_j.double_in_place(jh_j_cs.ns(|| format!("Double {}", i)))?;
                }

                let num_exponentiations = 2usize.pow(num_doubling);

                if j > num_exponentiations {
                    for i in 0..(j - num_exponentiations) {
                        jh_j = jh_j.add(jh_j_cs.ns(|| format!("Add: {}", i)), &h_j)?;
                    }
                }
                jh_j
            };

            // (z_i [+] j) * h_i,j
            let expected_record_view_key = zh_j.add(cs.ns(|| "expected record view key"), &jh_j)?;

            expected_record_view_key.enforce_equal(
                &mut cs.ns(|| "Check that declared and computed record view keys are equal"),
                &record_view_key_gadget,
            )?;

            // Construct c_j
            let c_j = h_j.add(cs.ns(|| "construct c_j"), &m_j)?;

            ciphertext.push(c_j);
        }

        Ok(GroupEncryptionCiphertextGadget {
            ciphertext,
            _engine: PhantomData,
            _group: PhantomData,
        })
    }
}
