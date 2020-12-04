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

use crate::{
    alloc_int_impl,
    curves::{Field, FpParameters, PrimeField},
    gadgets::{
        r1cs::{Assignment, ConstraintSystem, LinearCombination},
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            eq::{ConditionalEqGadget, EqGadget, EvaluateEqGadget},
            select::CondSelectGadget,
            uint::unsigned_integer::{UInt, UInt8},
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_utilities::{
    biginteger::{BigInteger, BigInteger256},
    bytes::ToBytes,
};

use std::{borrow::Borrow, cmp::Ordering};

/// Represents an interpretation of 128 `Boolean` objects as an
/// unsigned integer.
#[derive(Clone, Debug)]
pub struct UInt128 {
    // Least significant bit_gadget first
    pub bits: Vec<Boolean>,
    pub negated: bool,
    pub value: Option<u128>,
}

impl UInt128 {
    /// Construct a constant `UInt128` from a `u128`
    pub fn constant(value: u128) -> Self {
        let mut bits = Vec::with_capacity(128);

        let mut tmp = value;
        for _ in 0..128 {
            if tmp & 1 == 1 {
                bits.push(Boolean::constant(true))
            } else {
                bits.push(Boolean::constant(false))
            }

            tmp >>= 1;
        }

        Self {
            bits,
            negated: false,
            value: Some(value),
        }
    }
}

impl UInt for UInt128 {
    /// Returns the inverse UInt128
    fn negate(&self) -> Self {
        Self {
            bits: self.bits.clone(),
            negated: true,
            value: self.value,
        }
    }

    /// Returns true if all bits in this UInt128 are constant
    fn is_constant(&self) -> bool {
        let mut constant = true;

        // If any bits of self are allocated bits, return false
        for bit in &self.bits {
            match *bit {
                Boolean::Is(ref _bit) => constant = false,
                Boolean::Not(ref _bit) => constant = false,
                Boolean::Constant(_bit) => {}
            }
        }

        constant
    }

    /// Turns this `UInt128` into its little-endian byte order representation.
    fn to_bits_le(&self) -> Vec<Boolean> {
        self.bits.clone()
    }

    /// Converts a little-endian byte order representation of bits into a
    /// `UInt128`.
    fn from_bits_le(bits: &[Boolean]) -> Self {
        assert_eq!(bits.len(), 128);

        let bits = bits.to_vec();

        let mut value = Some(0u128);
        for b in bits.iter().rev() {
            if let Some(v) = value.as_mut() {
                *v <<= 1;
            }

            match *b {
                Boolean::Constant(b) => {
                    if b {
                        if let Some(v) = value.as_mut() {
                            *v |= 1;
                        }
                    }
                }
                Boolean::Is(ref b) => match b.get_value() {
                    Some(true) => {
                        if let Some(v) = value.as_mut() {
                            *v |= 1;
                        }
                    }
                    Some(false) => {}
                    None => value = None,
                },
                Boolean::Not(ref b) => match b.get_value() {
                    Some(false) => {
                        if let Some(v) = value.as_mut() {
                            *v |= 1;
                        }
                    }
                    Some(true) => {}
                    None => value = None,
                },
            }
        }

        Self {
            value,
            negated: false,
            bits,
        }
    }

    fn rotr(&self, by: usize) -> Self {
        let by = by % 128;

        let new_bits = self
            .bits
            .iter()
            .skip(by)
            .chain(self.bits.iter())
            .take(128)
            .cloned()
            .collect();

        Self {
            bits: new_bits,
            negated: false,
            value: self.value.map(|v| v.rotate_right(by as u32) as u128),
        }
    }

    /// XOR this `UInt128` with another `UInt128`
    fn xor<F: Field, CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let new_value = match (self.value, other.value) {
            (Some(a), Some(b)) => Some(a ^ b),
            _ => None,
        };

        let bits = self
            .bits
            .iter()
            .zip(other.bits.iter())
            .enumerate()
            .map(|(i, (a, b))| Boolean::xor(cs.ns(|| format!("xor of bit_gadget {}", i)), a, b))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            bits,
            negated: false,
            value: new_value,
        })
    }

    /// Perform modular addition of several `UInt128` objects.
    fn addmany<F: PrimeField, CS: ConstraintSystem<F>>(mut cs: CS, operands: &[Self]) -> Result<Self, SynthesisError> {
        // Make some arbitrary bounds for ourselves to avoid overflows
        // in the scalar field
        assert!(F::Parameters::MODULUS_BITS <= 253);
        assert!(operands.len() >= 2); // Weird trivial cases that should never happen

        // Compute the maximum value of the sum so we allocate enough bits for
        // the result
        let mut max_value = BigInteger256::from_u128(u128::max_value());
        max_value.muln(operands.len() as u32);

        // Keep track of the resulting value
        let mut big_result_value = Some(BigInteger256::default());

        // This is a linear combination that we will enforce to be "zero"
        let mut lc = LinearCombination::zero();

        let mut all_constants = true;

        // Iterate over the operands
        for op in operands {
            // Accumulate the value
            match op.value {
                Some(val) => {
                    // Subtract or add operand
                    if op.negated {
                        // Perform subtraction
                        big_result_value
                            .as_mut()
                            .map(|v| v.sub_noborrow(&BigInteger256::from_u128(val)));
                    } else {
                        // Perform addition
                        big_result_value
                            .as_mut()
                            .map(|v| v.add_nocarry(&BigInteger256::from_u128(val)));
                    }
                }
                None => {
                    // If any of our operands have unknown value, we won't
                    // know the value of the result
                    big_result_value = None;
                }
            }

            // Iterate over each bit_gadget of the operand and add the operand to
            // the linear combination
            let mut coeff = F::one();
            for bit in &op.bits {
                match *bit {
                    Boolean::Is(ref bit) => {
                        all_constants = false;

                        if op.negated {
                            // Subtract coeff * bit gadget
                            lc = lc - (coeff, bit.get_variable());
                        } else {
                            // Add coeff * bit_gadget
                            lc += (coeff, bit.get_variable());
                        }
                    }
                    Boolean::Not(ref bit) => {
                        all_constants = false;

                        if op.negated {
                            // subtract coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                            lc = lc - (coeff, CS::one()) + (coeff, bit.get_variable());
                        } else {
                            // Add coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                            lc = lc + (coeff, CS::one()) - (coeff, bit.get_variable());
                        }
                    }
                    Boolean::Constant(bit) => {
                        if bit {
                            if op.negated {
                                lc = lc - (coeff, CS::one());
                            } else {
                                lc += (coeff, CS::one());
                            }
                        }
                    }
                }

                coeff.double_in_place();
            }
        }

        // The value of the actual result is modulo 2^128
        let modular_value = big_result_value.map(|v| v.to_u128());

        if all_constants {
            if let Some(val) = modular_value {
                // We can just return a constant, rather than
                // unpacking the result into allocated bits.

                return Ok(Self::constant(val));
            }
        }

        // Storage area for the resulting bits
        let mut result_bits = vec![];

        // Allocate each bit_gadget of the result
        let mut coeff = F::one();
        let mut i = 0;
        while !max_value.is_zero() {
            // Allocate the bit_gadget
            let b = AllocatedBit::alloc(cs.ns(|| format!("result bit_gadget {}", i)), || {
                big_result_value.map(|v| v.get_bit(i)).get()
            })?;

            // Subtract this bit_gadget from the linear combination to ensure the sums
            // balance out
            lc = lc - (coeff, b.get_variable());

            // Discard carry bits that we don't care about
            if result_bits.len() < 128 {
                result_bits.push(b.into());
            }

            max_value.div2();
            i += 1;
            coeff.double_in_place();
        }

        // Enforce that the linear combination equals zero
        cs.enforce(|| "modular addition", |lc| lc, |lc| lc, |_| lc);

        Ok(Self {
            bits: result_bits,
            negated: false,
            value: modular_value,
        })
    }

    /// Perform modular subtraction of two `UInt128` objects.
    fn sub<F: PrimeField, CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        // pseudocode:
        //
        // a - b
        // a + (-b)

        Self::addmany(&mut cs.ns(|| "add_not"), &[self.clone(), other.negate()])
    }

    /// Perform unsafe subtraction of two `UInt128` objects which returns 0 if overflowed
    fn sub_unsafe<F: PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        match (self.value, other.value) {
            (Some(val1), Some(val2)) => {
                // Check for overflow
                if val1 < val2 {
                    // Instead of erroring, return 0

                    if Self::result_is_constant(&self, &other) {
                        // Return constant 0u128
                        Ok(Self::constant(0u128))
                    } else {
                        // Return allocated 0u128
                        let result_value = Some(0u128);
                        let modular_value = result_value.map(|v| v as u128);

                        // Storage area for the resulting bits
                        let mut result_bits = Vec::with_capacity(128);

                        // This is a linear combination that we will enforce to be "zero"
                        let mut lc = LinearCombination::zero();

                        // Allocate each bit_gadget of the result
                        let mut coeff = F::one();
                        for i in 0..128 {
                            // Allocate the bit_gadget
                            let b = AllocatedBit::alloc(cs.ns(|| format!("result bit_gadget {}", i)), || {
                                result_value.map(|v| (v >> i) & 1 == 1).get()
                            })?;

                            // Subtract this bit_gadget from the linear combination to ensure the sums
                            // balance out
                            lc = lc - (coeff, b.get_variable());

                            result_bits.push(b.into());

                            coeff.double_in_place();
                        }

                        // Enforce that the linear combination equals zero
                        cs.enforce(|| "unsafe subtraction", |lc| lc, |lc| lc, |_| lc);

                        Ok(Self {
                            bits: result_bits,
                            negated: false,
                            value: modular_value,
                        })
                    }
                } else {
                    // Perform subtraction
                    self.sub(&mut cs.ns(|| ""), &other)
                }
            }
            (_, _) => {
                // If either of our operands have unknown value, we won't
                // know the value of the result
                Err(SynthesisError::AssignmentMissing)
            }
        }
    }

    /// Bitwise multiplication of two `UInt128` objects.
    /// Reference: https://en.wikipedia.org/wiki/Binary_multiplier
    fn mul<F: PrimeField, CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        // pseudocode:
        //
        // res = 0;
        // shifted_self = self;
        // for bit in other.bits {
        //   if bit {
        //     res += shifted_self;
        //   }
        //   shifted_self = shifted_self << 1;
        // }
        // return res

        let is_constant = Boolean::constant(Self::result_is_constant(&self, &other));
        let constant_result = Self::constant(0u128);
        let allocated_result = Self::alloc(&mut cs.ns(|| "allocated_1u128"), || Ok(0u128))?;
        let zero_result = Self::conditionally_select(
            &mut cs.ns(|| "constant_or_allocated"),
            &is_constant,
            &constant_result,
            &allocated_result,
        )?;

        let mut left_shift = self.clone();

        let partial_products = other
            .bits
            .iter()
            .enumerate()
            .map(|(i, bit)| {
                let current_left_shift = left_shift.clone();
                left_shift = Self::addmany(&mut cs.ns(|| format!("shift_left_{}", i)), &[
                    left_shift.clone(),
                    left_shift.clone(),
                ])
                .unwrap();

                Self::conditionally_select(
                    &mut cs.ns(|| format!("calculate_product_{}", i)),
                    &bit,
                    &current_left_shift,
                    &zero_result,
                )
                .unwrap()
            })
            .collect::<Vec<Self>>();

        Self::addmany(&mut cs.ns(|| "partial_products"), &partial_products)
    }

    /// Perform long division of two `UInt128` objects.
    /// Reference: https://en.wikipedia.org/wiki/Division_algorithm
    fn div<F: PrimeField, CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        // pseudocode:
        //
        // if D = 0 then error(DivisionByZeroException) end
        // Q := 0                  -- Initialize quotient and remainder to zero
        // R := 0
        // for i := n − 1 .. 0 do  -- Where n is number of bits in N
        //   R := R << 1           -- Left-shift R by 1 bit
        //   R(0) := N(i)          -- Set the least-significant bit of R equal to bit i of the numerator
        //   if R ≥ D then
        //     R := R − D
        //     Q(i) := 1
        //   end
        // end

        if other.eq(&Self::constant(0u128)) {
            return Err(SynthesisError::DivisionByZero);
        }

        let is_constant = Boolean::constant(Self::result_is_constant(&self, &other));

        let allocated_true = Boolean::from(AllocatedBit::alloc(&mut cs.ns(|| "true"), || Ok(true)).unwrap());
        let true_bit = Boolean::conditionally_select(
            &mut cs.ns(|| "constant_or_allocated_true"),
            &is_constant,
            &Boolean::constant(true),
            &allocated_true,
        )?;

        let allocated_one = Self::alloc(&mut cs.ns(|| "one"), || Ok(1u128))?;
        let one = Self::conditionally_select(
            &mut cs.ns(|| "constant_or_allocated_1u128"),
            &is_constant,
            &Self::constant(1u128),
            &allocated_one,
        )?;

        let allocated_zero = Self::alloc(&mut cs.ns(|| "zero"), || Ok(0u128))?;
        let zero = Self::conditionally_select(
            &mut cs.ns(|| "constant_or_allocated_0u128"),
            &is_constant,
            &Self::constant(0u128),
            &allocated_zero,
        )?;

        let self_is_zero = Boolean::Constant(self.eq(&Self::constant(0u128)));
        let mut quotient = zero.clone();
        let mut remainder = zero;

        for (i, bit) in self.bits.iter().rev().enumerate() {
            // Left shift remainder by 1
            remainder = Self::addmany(&mut cs.ns(|| format!("shift_left_{}", i)), &[
                remainder.clone(),
                remainder.clone(),
            ])?;

            // Set the least-significant bit of remainder to bit i of the numerator
            let bit_is_true = Boolean::constant(bit.eq(&Boolean::constant(true)));
            let new_remainder = Self::addmany(&mut cs.ns(|| format!("set_remainder_bit_{}", i)), &[
                remainder.clone(),
                one.clone(),
            ])?;

            remainder = Self::conditionally_select(
                &mut cs.ns(|| format!("increment_or_remainder_{}", i)),
                &bit_is_true,
                &new_remainder,
                &remainder,
            )?;

            // Greater than or equal to:
            //   R >= D
            //   (R == D) || (R > D)
            //   (R == D) || ((R !=D) && ((R - D) != 0))
            //
            //  (R > D)                     checks subtraction overflow before evaluation
            //  (R != D) && ((R - D) != 0)  instead evaluate subtraction and check for overflow after

            let no_remainder = Boolean::constant(remainder.eq(&other));
            let subtraction = remainder.sub_unsafe(&mut cs.ns(|| format!("subtract_divisor_{}", i)), &other)?;
            let sub_is_zero = Boolean::constant(subtraction.eq(&Self::constant(0)));
            let cond1 = Boolean::and(
                &mut cs.ns(|| format!("cond_1_{}", i)),
                &no_remainder.not(),
                &sub_is_zero.not(),
            )?;
            let cond2 = Boolean::or(&mut cs.ns(|| format!("cond_2_{}", i)), &no_remainder, &cond1)?;

            remainder = Self::conditionally_select(
                &mut cs.ns(|| format!("subtract_or_same_{}", i)),
                &cond2,
                &subtraction,
                &remainder,
            )?;

            let index = 127 - i as usize;
            let bit_value = 1u128 << (index as u128);
            let mut new_quotient = quotient.clone();
            new_quotient.bits[index] = true_bit;
            new_quotient.value = Some(new_quotient.value.unwrap() + bit_value);

            quotient = Self::conditionally_select(
                &mut cs.ns(|| format!("set_bit_or_same_{}", i)),
                &cond2,
                &new_quotient,
                &quotient,
            )?;
        }
        Self::conditionally_select(&mut cs.ns(|| "self_or_quotient"), &self_is_zero, self, &quotient)
    }

    /// Bitwise multiplication of two `UInt128` objects.
    /// Reference: /snarkOS/models/src/curves/field.rs
    fn pow<F: Field + PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        // let mut res = Self::one();
        //
        // let mut found_one = false;
        //
        // for i in BitIterator::new(exp) {
        //     if !found_one {
        //         if i {
        //             found_one = true;
        //         } else {
        //             continue;
        //         }
        //     }
        //
        //     res.square_in_place();
        //
        //     if i {
        //         res *= self;
        //     }
        // }
        // res

        let is_constant = Boolean::constant(Self::result_is_constant(&self, &other));
        let constant_result = Self::constant(1u128);
        let allocated_result = Self::alloc(&mut cs.ns(|| "allocated_1u128"), || Ok(1u128))?;
        let mut result = Self::conditionally_select(
            &mut cs.ns(|| "constant_or_allocated"),
            &is_constant,
            &constant_result,
            &allocated_result,
        )?;

        for (i, bit) in other.bits.iter().rev().enumerate() {
            let found_one = Boolean::Constant(result.eq(&Self::constant(1u128)));
            let cond1 = Boolean::and(cs.ns(|| format!("found_one_{}", i)), &bit.not(), &found_one)?;
            let square = result.mul(cs.ns(|| format!("square_{}", i)), &result).unwrap();

            result = Self::conditionally_select(
                &mut cs.ns(|| format!("result_or_sqaure_{}", i)),
                &cond1,
                &result,
                &square,
            )?;

            let mul_by_self = result.mul(cs.ns(|| format!("multiply_by_self_{}", i)), &self).unwrap();

            result = Self::conditionally_select(
                &mut cs.ns(|| format!("mul_by_self_or_result_{}", i)),
                &bit,
                &mul_by_self,
                &result,
            )?;
        }

        Ok(result)
    }
}

impl PartialEq for UInt128 {
    fn eq(&self, other: &Self) -> bool {
        self.value.is_some() && self.value == other.value
    }
}

impl Eq for UInt128 {}

impl PartialOrd for UInt128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Option::from(self.value.cmp(&other.value))
    }
}

impl<F: PrimeField> EvaluateEqGadget<F> for UInt128 {
    fn evaluate_equal<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Boolean, SynthesisError> {
        let mut result = Boolean::constant(true);
        for (i, (a, b)) in self.bits.iter().zip(&other.bits).enumerate() {
            let equal = a.evaluate_equal(&mut cs.ns(|| format!("u128 evaluate equality for {}-th bit", i)), b)?;

            result = Boolean::and(
                &mut cs.ns(|| format!("u128 and result for {}-th bit", i)),
                &equal,
                &result,
            )?;
        }

        Ok(result)
    }
}

impl<F: Field> EqGadget<F> for UInt128 {}

impl<F: Field> ConditionalEqGadget<F> for UInt128 {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (a, b)) in self.bits.iter().zip(&other.bits).enumerate() {
            a.conditional_enforce_equal(&mut cs.ns(|| format!("uint128_equal_{}", i)), b, condition)?;
        }
        Ok(())
    }

    fn cost() -> usize {
        128 * <Boolean as ConditionalEqGadget<F>>::cost()
    }
}

impl<F: PrimeField> CondSelectGadget<F> for UInt128 {
    fn conditionally_select<CS: ConstraintSystem<F>>(
        mut cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError> {
        if let Boolean::Constant(cond) = *cond {
            if cond { Ok(first.clone()) } else { Ok(second.clone()) }
        } else {
            let mut is_negated = false;

            let result_val = cond.get_value().and_then(|c| {
                if c {
                    is_negated = first.negated;
                    first.value
                } else {
                    is_negated = second.negated;
                    second.value
                }
            });

            let mut result = Self::alloc(cs.ns(|| "cond_select_result"), || result_val.get())?;

            result.negated = is_negated;

            let expected_bits = first
                .bits
                .iter()
                .zip(&second.bits)
                .enumerate()
                .map(|(i, (a, b))| {
                    Boolean::conditionally_select(&mut cs.ns(|| format!("uint128_cond_select_{}", i)), cond, a, b)
                        .unwrap()
                })
                .collect::<Vec<Boolean>>();

            for (i, (actual, expected)) in result.to_bits_le().iter().zip(expected_bits.iter()).enumerate() {
                actual.enforce_equal(&mut cs.ns(|| format!("selected_result_bit_{}", i)), expected)?;
            }

            Ok(result)
        }
    }

    fn cost() -> usize {
        128 * (<Boolean as ConditionalEqGadget<F>>::cost() + <Boolean as CondSelectGadget<F>>::cost())
    }
}

alloc_int_impl!(UInt128, u128, 128);

impl<F: Field> ToBytesGadget<F> for UInt128 {
    #[inline]
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let value_chunks = match self.value.map(|val| {
            let mut bytes = [0u8; 16];
            val.write(bytes.as_mut()).unwrap();
            bytes
        }) {
            Some(chunks) => [Some(chunks[0]), Some(chunks[1]), Some(chunks[2]), Some(chunks[3])],
            None => [None, None, None, None],
        };
        let mut bytes = Vec::new();
        for (i, chunk8) in self.to_bits_le().chunks(8).enumerate() {
            let byte = UInt8 {
                bits: chunk8.to_vec(),
                negated: false,
                value: value_chunks[i],
            };
            bytes.push(byte);
        }

        Ok(bytes)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}
