use crate::{
    curves::{fp_parameters::FpParameters, to_field_vec::ToConstraintField, Field, PrimeField},
    gadgets::{
        curves::FpGadget,
        r1cs::{Assignment, ConstraintSystem, LinearCombination},
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            eq::{ConditionalEqGadget, EqGadget},
            select::CondSelectGadget,
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_utilities::bytes::ToBytes;

use core::borrow::Borrow;
use snarkos_errors::gadgets::SynthesisError;
use std::{cmp::Ordering, fmt::Debug};

uint_impl!(UInt8, u8, 8);
uint_impl!(UInt16, u16, 16);
uint_impl!(UInt32, u32, 32);
uint_impl!(UInt64, u64, 64);

pub trait UInt: Debug + Clone + PartialOrd + Eq + PartialEq {
    /// Returns the inverse `UInt`
    fn negate(&self) -> Self;

    /// Returns true if all bits in this `UInt` are constant
    fn is_constant(&self) -> bool;

    /// Returns true if both `UInt` objects have constant bits
    fn result_is_constant(first: &Self, second: &Self) -> bool {
        // If any bits of first are allocated bits, return false
        if !first.is_constant() {
            return false;
        }

        // If any bits of second are allocated bits, return false
        second.is_constant()
    }

    /// Turns this `UInt` into its little-endian byte order representation.
    /// LSB-first means that we can easily get the corresponding field element
    /// via double and add.
    fn to_bits_le(&self) -> Vec<Boolean>;

    /// Converts a little-endian byte order representation of bits into a
    /// `UInt`.
    fn from_bits_le(bits: &[Boolean]) -> Self;

    /// Rotate self bits by size
    fn rotr(&self, by: usize) -> Self;

    /// XOR this `UInt` with another `UInt`
    fn xor<F: Field, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    /// Perform modular addition of several `UInt` objects.
    fn addmany<F: PrimeField, CS: ConstraintSystem<F>>(cs: CS, operands: &[Self]) -> Result<Self, SynthesisError>;

    /// Perform modular subtraction of two `UInt` objects.
    fn sub<F: PrimeField, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    /// Perform unsafe subtraction of two `UInt` objects which returns 0 if overflowed
    fn sub_unsafe<F: PrimeField, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    /// Perform Bitwise multiplication of two `UInt` objects.
    /// Reference: https://en.wikipedia.org/wiki/Binary_multiplier
    fn mul<F: PrimeField, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    /// Perform long division of two `UInt` objects.
    /// Reference: https://en.wikipedia.org/wiki/Division_algorithm
    fn div<F: PrimeField, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    /// Bitwise exponentiation of two `UInt64` objects.
    /// Reference: /snarkOS/models/src/curves/field.rs
    fn pow<F: Field + PrimeField, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self)
    -> Result<Self, SynthesisError>;
}

// These methods are used throughout snarkos-gadgets exclusively by UInt8
impl UInt8 {
    /// Construct a constant vector of `UInt8` from a vector of `u8`
    pub fn constant_vec(values: &[u8]) -> Vec<Self> {
        let mut result = Vec::new();
        for value in values {
            result.push(UInt8::constant(*value));
        }
        result
    }

    pub fn alloc_vec<F, CS, T>(mut cs: CS, values: &[T]) -> Result<Vec<Self>, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
        T: Into<Option<u8>> + Copy,
    {
        let mut output_vec = Vec::with_capacity(values.len());
        for (i, value) in values.into_iter().enumerate() {
            let byte: Option<u8> = Into::into(*value);
            let alloc_byte = Self::alloc(&mut cs.ns(|| format!("byte_{}", i)), || byte.get())?;
            output_vec.push(alloc_byte);
        }
        Ok(output_vec)
    }

    /// Allocates a vector of `u8`'s by first converting (chunks of) them to
    /// `F` elements, (thus reducing the number of input allocations),
    /// and then converts this list of `F` gadgets back into
    /// bytes.
    pub fn alloc_input_vec<F, CS>(mut cs: CS, values: &[u8]) -> Result<Vec<Self>, SynthesisError>
    where
        F: PrimeField,
        CS: ConstraintSystem<F>,
    {
        let values_len = values.len();
        let field_elements: Vec<F> = ToConstraintField::<F>::to_field_elements(values).unwrap();

        let max_size = 8 * (F::Params::CAPACITY / 8) as usize;
        let mut allocated_bits = Vec::new();
        for (i, field_element) in field_elements.into_iter().enumerate() {
            let fe = FpGadget::alloc_input(&mut cs.ns(|| format!("Field element {}", i)), || Ok(field_element))?;
            let mut fe_bits = fe.to_bits(cs.ns(|| format!("Convert fe to bits {}", i)))?;
            // FpGadget::to_bits outputs a big-endian binary representation of
            // fe_gadget's value, so we have to reverse it to get the little-endian
            // form.
            fe_bits.reverse();

            // Remove the most significant bit, because we know it should be zero
            // because `values.to_field_elements()` only
            // packs field elements up to the penultimate bit.
            // That is, the most significant bit (`F::NUM_BITS`-th bit) is
            // unset, so we can just pop it off.
            allocated_bits.extend_from_slice(&fe_bits[0..max_size]);
        }

        // Chunk up slices of 8 bit into bytes.
        Ok(allocated_bits[0..8 * values_len]
            .chunks(8)
            .map(Self::from_bits_le)
            .collect())
    }
}
