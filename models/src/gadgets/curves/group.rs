use crate::{
    curves::{AffineCurve, Field, Group, ProjectiveCurve},
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{EqGadget, NEqGadget},
            select::CondSelectGadget,
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use itertools::Itertools;
use std::{borrow::Borrow, fmt::Debug};

pub trait GroupGadget<G: Group, F: Field>:
    Sized
    + ToBytesGadget<F>
    + NEqGadget<F>
    + EqGadget<F>
    + ToBitsGadget<F>
    + CondSelectGadget<F>
    + AllocGadget<G, F>
    + Clone
    + Debug
{
    type Value: Debug;
    type Variable;

    fn get_value(&self) -> Option<Self::Value>;

    fn get_variable(&self) -> Self::Variable;

    fn zero<CS: ConstraintSystem<F>>(cs: CS) -> Result<Self, SynthesisError>;

    fn add<CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    fn sub<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let neg_other = other.negate(cs.ns(|| "Negate other"))?;
        self.add(cs.ns(|| "Self - other"), &neg_other)
    }

    fn add_constant<CS: ConstraintSystem<F>>(&self, cs: CS, other: &G) -> Result<Self, SynthesisError>;

    fn sub_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &G) -> Result<Self, SynthesisError> {
        let neg_other = -(*other);
        self.add_constant(cs.ns(|| "Self - other"), &neg_other)
    }

    fn double_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS) -> Result<(), SynthesisError>;

    fn negate<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Self, SynthesisError>;

    /// Inputs must be specified in *little-endian* form.
    /// If the addition law is incomplete for the identity element,
    /// `result` must not be the identity element.
    fn mul_bits<'a, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        result: &Self,
        bits: impl Iterator<Item = &'a Boolean>,
    ) -> Result<Self, SynthesisError> {
        let mut base = self.clone();
        let mut result = result.clone();
        for (i, bit) in bits.enumerate() {
            let new_encoded = result.add(&mut cs.ns(|| format!("Add {}-th power", i)), &base)?;
            result = Self::conditionally_select(
                &mut cs.ns(|| format!("Select {}", i)),
                bit.borrow(),
                &new_encoded,
                &result,
            )?;
            base.double_in_place(&mut cs.ns(|| format!("{}-th Doubling", i)))?;
        }
        Ok(result)
    }

    fn precomputed_base_scalar_mul<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
    ) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a G)>,
        B: Borrow<Boolean>,
        G: 'a,
    {
        for (i, (bit, base_power)) in scalar_bits_with_base_powers.enumerate() {
            let new_encoded = self.add_constant(&mut cs.ns(|| format!("Add {}-th base power", i)), &base_power)?;
            *self = Self::conditionally_select(
                &mut cs.ns(|| format!("Conditional Select {}", i)),
                bit.borrow(),
                &new_encoded,
                &self,
            )?;
        }
        Ok(())
    }

    fn precomputed_base_symmetric_scalar_mul<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
    ) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a G)>,
        B: Borrow<Boolean>,
        G: 'a,
    {
        for (i, (bit, base_power)) in scalar_bits_with_base_powers.enumerate() {
            let new_encoded_plus =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power plus", i)), &base_power)?;
            let new_encoded_minus = self.add_constant(
                &mut cs.ns(|| format!("Add {}-th base power minus", i)),
                &base_power.neg(),
            )?;
            *self = Self::conditionally_select(
                &mut cs.ns(|| format!("Conditional Select {}", i)),
                bit.borrow(),
                &new_encoded_plus,
                &new_encoded_minus,
            )?;
        }
        Ok(())
    }

    fn precomputed_base_scalar_mul_masked<'a, CS, I, B>(&mut self, _: CS, _: I, _: I) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a G)>,
        B: Borrow<Boolean>,
        G: 'a,
    {
        Err(SynthesisError::AssignmentMissing)
    }

    fn precomputed_base_3_bit_signed_digit_scalar_mul<'a, CS, I, J, B>(
        _: CS,
        _: &[B],
        _: &[J],
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Borrow<[Boolean]>,
        J: Borrow<[I]>,
        B: Borrow<[G]>,
    {
        Err(SynthesisError::AssignmentMissing)
    }

    fn precomputed_base_multiscalar_mul<'a, CS, T, I, B>(
        mut cs: CS,
        bases: &[B],
        scalars: I,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        T: 'a + ToBitsGadget<F> + ?Sized,
        I: Iterator<Item = &'a T>,
        B: Borrow<[G]>,
    {
        let mut result = Self::zero(&mut cs.ns(|| "Declare Result"))?;
        // Compute ∏(h_i^{m_i}) for all i.
        for (i, (bits, base_powers)) in scalars.zip_eq(bases).enumerate() {
            let base_powers = base_powers.borrow();
            let bits = bits.to_bits(&mut cs.ns(|| format!("Convert Scalar {} to bits", i)))?;
            result.precomputed_base_scalar_mul(cs.ns(|| format!("Chunk {}", i)), bits.iter().zip_eq(base_powers))?;
        }
        Ok(result)
    }

    fn precomputed_base_symmetric_multiscalar_mul<'a, CS, T, I, B>(
        mut cs: CS,
        bases: &[B],
        scalars: I,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        T: 'a + ToBitsGadget<F> + ?Sized,
        I: Iterator<Item = &'a T>,
        B: Borrow<[G]>,
    {
        let mut result = Self::zero(&mut cs.ns(|| "Declare Result"))?;
        // Compute ∏(h_i^{1  - 2*m_i}) for all i.
        for (i, (bits, base_powers)) in scalars.zip_eq(bases).enumerate() {
            let base_powers = base_powers.borrow();
            let bits = bits.to_bits(&mut cs.ns(|| format!("Convert Scalar {} to bits", i)))?;

            result.precomputed_base_symmetric_scalar_mul(
                cs.ns(|| format!("Chunk {}", i)),
                bits.iter().zip_eq(base_powers),
            )?;
        }
        Ok(result)
    }

    /// Compute ∏((h_i^{-1} * 1[p_i = 0] + h_i * 1[p_i = 1])^{1 - m_i \xor p_i})((g_i h_i^{-1} *
    /// 1[p_i = 0] + g_i^{-1} h_i * 1[p_i = 1])^{m_i \xor p_i}) for all i, m_i
    /// being the scalars, p_i being the masks, h_i being the symmetric Pedersen bases and g_i the
    /// Pedersen bases.
    fn precomputed_base_multiscalar_mul_masked<'a, CS, T, I, B>(
        mut cs: CS,
        bases: &[B],
        scalars: I,
        mask_bases: &[B],
        masks: I,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        T: 'a + ToBitsGadget<F> + ?Sized,
        I: Iterator<Item = &'a T>,
        B: Borrow<[G]>,
    {
        let mut result = Self::zero(&mut cs.ns(|| "Declare Result"))?;
        for (i, (((scalar, mask), base_powers), mask_powers)) in
            scalars.zip_eq(masks).zip_eq(bases).zip_eq(mask_bases).enumerate()
        {
            let base_powers = base_powers.borrow();
            let mask_powers = mask_powers.borrow();
            let scalar_bits = scalar.to_bits(&mut cs.ns(|| format!("Convert scalar {} to bits", i)))?;
            let mask_bits = mask.to_bits(&mut cs.ns(|| format!("Convert mask {} to bits", i)))?;

            let scalar_bits_with_base_powers = scalar_bits.into_iter().zip_eq(base_powers);
            let mask_bits_with_mask_powers = mask_bits.into_iter().zip_eq(mask_powers);

            result.precomputed_base_scalar_mul_masked(
                cs.ns(|| format!("Chunk {}", i)),
                scalar_bits_with_base_powers,
                mask_bits_with_mask_powers,
            )?;
        }
        Ok(result)
    }

    fn cost_of_add() -> usize;

    fn cost_of_double() -> usize;
}

pub trait CompressedGroupGadget<G: Group + ProjectiveCurve, F: Field>: GroupGadget<G, F> {
    type BaseFieldGadget: ToBytesGadget<F>
        + EqGadget<F>
        + CondSelectGadget<F>
        + AllocGadget<<G::Affine as AffineCurve>::BaseField, F>
        + Clone
        + Debug;

    fn to_x_coordinate(&self) -> Self::BaseFieldGadget;
}
