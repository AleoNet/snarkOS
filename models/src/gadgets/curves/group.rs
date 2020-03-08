use crate::{
    curves::{Field, Group},
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
        let mut power = self.clone();
        let mut result = result.clone();
        for (i, bit) in bits.enumerate() {
            let new_encoded = result.add(&mut cs.ns(|| format!("Add {}-th power", i)), &power)?;
            result = Self::conditionally_select(
                &mut cs.ns(|| format!("Select {}", i)),
                bit.borrow(),
                &new_encoded,
                &result,
            )?;
            power.double_in_place(&mut cs.ns(|| format!("{}-th Doubling", i)))?;
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
        for (i, (bits, base_powers)) in scalars.zip(bases).enumerate() {
            let base_powers = base_powers.borrow();
            let bits = bits.to_bits(&mut cs.ns(|| format!("Convert Scalar {} to bits", i)))?;
            result.precomputed_base_scalar_mul(cs.ns(|| format!("Chunk {}", i)), bits.iter().zip(base_powers))?;
        }
        Ok(result)
    }

    fn cost_of_add() -> usize;

    fn cost_of_double() -> usize;
}
