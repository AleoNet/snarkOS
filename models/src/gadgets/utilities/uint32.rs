use crate::{
    curves::{Field, FpParameters, PrimeField},
    gadgets::{
        r1cs::{Assignment, ConstraintSystem, LinearCombination},
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            eq::ConditionalEqGadget,
            uint8::UInt8,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_utilities::bytes::ToBytes;
use std::ops::{Shl, Shr};

/// Represents an interpretation of 32 `Boolean` objects as an
/// unsigned integer.
#[derive(Clone, Debug)]
pub struct UInt32 {
    // Least significant bit_gadget first
    pub bits: Vec<Boolean>,
    pub value: Option<u32>,
}

impl UInt32 {
    /// Construct a constant `UInt32` from a `u32`
    pub fn constant(value: u32) -> Self {
        let mut bits = Vec::with_capacity(32);

        let mut tmp = value;
        for _ in 0..32 {
            if tmp & 1 == 1 {
                bits.push(Boolean::constant(true))
            } else {
                bits.push(Boolean::constant(false))
            }

            tmp >>= 1;
        }

        UInt32 {
            bits,
            value: Some(value),
        }
    }

    /// Allocate a `UInt32` in the constraint system
    pub fn alloc<F, CS>(mut cs: CS, value: Option<u32>) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
        let values = match value {
            Some(mut val) => {
                let mut v = Vec::with_capacity(32);

                for _ in 0..32 {
                    v.push(Some(val & 1 == 1));
                    val >>= 1;
                }

                v
            }
            None => vec![None; 32],
        };

        let bits = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                Ok(Boolean::from(AllocatedBit::alloc(
                    cs.ns(|| format!("allocated bit_gadget {}", i)),
                    || v.get(),
                )?))
            })
            .collect::<Result<Vec<_>, SynthesisError>>()?;

        Ok(UInt32 { bits, value })
    }

    /// Turns this `UInt32` into its little-endian byte order representation.
    pub fn to_bits_le(&self) -> Vec<Boolean> {
        self.bits.clone()
    }

    /// Converts a little-endian byte order representation of bits into a
    /// `UInt32`.
    pub fn from_bits_le(bits: &[Boolean]) -> Self {
        assert_eq!(bits.len(), 32);

        let bits = bits.to_vec();

        let mut value = Some(0u32);
        for b in bits.iter().rev() {
            value.as_mut().map(|v| *v <<= 1);

            match b {
                &Boolean::Constant(b) => {
                    if b {
                        value.as_mut().map(|v| *v |= 1);
                    }
                }
                &Boolean::Is(ref b) => match b.get_value() {
                    Some(true) => {
                        value.as_mut().map(|v| *v |= 1);
                    }
                    Some(false) => {}
                    None => value = None,
                },
                &Boolean::Not(ref b) => match b.get_value() {
                    Some(false) => {
                        value.as_mut().map(|v| *v |= 1);
                    }
                    Some(true) => {}
                    None => value = None,
                },
            }
        }

        Self { value, bits }
    }

    pub fn not(&self) -> Self {
        let value = match self.value {
            Some(a) => Some(!a),
            _ => None,
        };

        let bits = self.bits.iter().map(|a| a.not()).collect();

        UInt32 { bits, value }
    }

    pub fn rotr(&self, by: usize) -> Self {
        let by = by % 32;

        let new_bits = self
            .bits
            .iter()
            .skip(by)
            .chain(self.bits.iter())
            .take(32)
            .cloned()
            .collect();

        UInt32 {
            bits: new_bits,
            value: self.value.map(|v| v.rotate_right(by as u32)),
        }
    }

    /// Shifts out by higher order bits. TODO: implement a checking shl
    pub fn shl(&self, by: usize) -> Self {
        let by = by % 32;
        let zero = UInt32::constant(0);

        let new_bits = zero
            .bits
            .iter()
            .take(by)
            .chain(self.bits.iter())
            .take(32)
            .cloned()
            .collect();

        UInt32 {
            bits: new_bits,
            value: self.value.map(|v| v.shl(by as u32)),
        }
    }

    /// Shifts out by lower order bits. TODO: implement a checking shr
    pub fn shr(&self, by: usize) -> Self {
        let by = by % 32;
        let zero = UInt32::constant(0);

        let new_bits = self
            .bits
            .iter()
            .skip(by)
            .chain(zero.bits.iter())
            .take(32)
            .cloned()
            .collect();

        UInt32 {
            bits: new_bits,
            value: self.value.map(|v| v.shr(by as u32)),
        }
    }

    pub fn and<F: Field, CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let value = match (self.value, other.value) {
            (Some(a), Some(b)) => Some(a & b),
            _ => None,
        };

        let bits = self
            .bits
            .iter()
            .zip(other.bits.iter())
            .enumerate()
            .map(|(i, (a, b))| Boolean::and(cs.ns(|| format!("and of bit gadget {}", i)), a, b))
            .collect::<Result<_, _>>()?;

        Ok(UInt32 { bits, value })
    }

    /// XOR this `UInt32` with another `UInt32`
    pub fn xor<F, CS>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
    {
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

        Ok(UInt32 { bits, value: new_value })
    }

    /// Bitwise exponentiation
    pub fn pows<F: Field + PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        let mut power = self.clone();
        let mut result = UInt32::constant(1);

        other.bits.iter().enumerate().for_each(|(i, bit)| {
            if bit.get_value() == Some(true) {
                result = result
                    .mul(cs.ns(|| format!("multiply by power {}", i)), &power)
                    .unwrap();
            }
            power = power.mul(cs.ns(|| format!("next power {}", i)), &power).unwrap();
        });

        Ok(result)
    }

    /// Perform modular addition of several `UInt32` objects.
    pub fn addmany<F, CS>(mut cs: CS, operands: &[Self]) -> Result<Self, SynthesisError>
    where
        F: PrimeField,
        CS: ConstraintSystem<F>,
    {
        // Make some arbitrary bounds for ourselves to avoid overflows
        // in the scalar field
        assert!(F::Params::MODULUS_BITS >= 64);
        assert!(operands.len() >= 2); // Weird trivial cases that should never happen
        assert!(operands.len() <= 10);

        // Compute the maximum value of the sum so we allocate enough bits for
        // the result
        let mut max_value = (operands.len() as u64) * u64::from(u32::max_value());

        // Keep track of the resulting value
        let mut result_value = Some(0u64);

        // This is a linear combination that we will enforce to be "zero"
        let mut lc = LinearCombination::zero();

        let mut all_constants = true;

        // Iterate over the operands
        for op in operands {
            // Accumulate the value
            match op.value {
                Some(val) => {
                    result_value.as_mut().map(|v| *v += u64::from(val));
                }
                None => {
                    // If any of our operands have unknown value, we won't
                    // know the value of the result
                    result_value = None;
                }
            }

            // Iterate over each bit_gadget of the operand and add the operand to
            // the linear combination
            let mut coeff = F::one();
            for bit in &op.bits {
                match *bit {
                    Boolean::Is(ref bit) => {
                        all_constants = false;

                        // Add coeff * bit_gadget
                        lc = lc + (coeff, bit.get_variable());
                    }
                    Boolean::Not(ref bit) => {
                        all_constants = false;

                        // Add coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                        lc = lc + (coeff, CS::one()) - (coeff, bit.get_variable());
                    }
                    Boolean::Constant(bit) => {
                        if bit {
                            lc = lc + (coeff, CS::one());
                        }
                    }
                }

                coeff.double_in_place();
            }
        }

        // The value of the actual result is modulo 2^32
        let modular_value = result_value.map(|v| v as u32);

        if all_constants && modular_value.is_some() {
            // We can just return a constant, rather than
            // unpacking the result into allocated bits.

            return Ok(UInt32::constant(modular_value.unwrap()));
        }

        // Storage area for the resulting bits
        let mut result_bits = vec![];

        // Allocate each bit_gadget of the result
        let mut coeff = F::one();
        let mut i = 0;
        while max_value != 0 {
            // Allocate the bit_gadget
            let b = AllocatedBit::alloc(cs.ns(|| format!("result bit_gadget {}", i)), || {
                result_value.map(|v| (v >> i) & 1 == 1).get()
            })?;

            // Subtract this bit_gadget from the linear combination to ensure the sums
            // balance out
            lc = lc - (coeff, b.get_variable());

            result_bits.push(b.into());

            max_value >>= 1;
            i += 1;
            coeff.double_in_place();
        }

        // Enforce that the linear combination equals zero
        cs.enforce(|| "modular addition", |lc| lc, |lc| lc, |_| lc);

        // Discard carry bits that we don't care about
        result_bits.truncate(32);

        Ok(UInt32 {
            bits: result_bits,
            value: modular_value,
        })
    }

    /// Perform modular subtraction of two `UInt32` objects.
    pub fn sub<F: PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        // Make some arbitrary bounds for ourselves to avoid overflows
        // in the scalar field
        assert!(F::Params::MODULUS_BITS >= 64);

        // Compute the maximum value of the sum so we allocate enough bits for
        // the result
        let mut max_value = u64::from(u32::max_value());

        // Evalue the self value
        let mut result_value = match self.value {
            Some(value) => Some(u64::from(value)),
            None => None,
        };

        // This is a linear combination that we will enforce to be "zero"
        let mut lc = LinearCombination::zero();

        let mut constant = true;

        // Evaluate the other value
        match other.value {
            Some(value) => {
                // Perform subtraction. If there is overflow this will fail.
                result_value.as_mut().map(|v| *v -= u64::from(value));
            }
            None => {
                result_value = None;
            }
        }

        // Iterate over each bit_gadget of self and add self to the linear combination
        let mut coeff = F::one();
        for bit in &self.bits {
            match *bit {
                Boolean::Is(ref bit) => {
                    constant = false;

                    // Add coeff * bit gadget
                    lc = lc + (coeff, bit.get_variable());
                }
                Boolean::Not(ref bit) => {
                    constant = false;

                    // Add coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                    lc = lc + (coeff, CS::one()) - (coeff, bit.get_variable());
                }
                Boolean::Constant(bit) => {
                    if bit {
                        lc = lc + (coeff, CS::one());
                    }
                }
            }

            coeff.double_in_place();
        }

        // Iterate over each bit_gadget of other and subtract other from the linear combination
        let mut coeff = F::one();
        for bit in &other.bits {
            match *bit {
                Boolean::Is(ref bit) => {
                    constant = false;

                    // Subtract coeff * bit_gadget
                    lc = lc - (coeff, bit.get_variable());
                }
                Boolean::Not(ref bit) => {
                    constant = false;

                    // Subtact coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                    lc = lc - (coeff, CS::one()) - (coeff, bit.get_variable());
                }
                Boolean::Constant(bit) => {
                    if bit {
                        lc = lc - (coeff, CS::one());
                    }
                }
            }

            coeff.double_in_place();
        }

        // The value of the actual result is moduluo 2^32
        let modular_value = result_value.map(|v| v as u32);

        if constant && modular_value.is_some() {
            // Return constant

            return Ok(UInt32::constant(modular_value.unwrap()));
        }

        // Storage for resulting bits
        let mut result_bits = vec![];

        // Allocate each bit gadget of the result
        let mut coeff = F::one();
        let mut i = 0;
        while max_value != 0 {
            // Allocate the bit_gadget
            let b = AllocatedBit::alloc(cs.ns(|| format!("subtraction result bit gadget {}", i)), || {
                result_value.map(|v| (v >> i) & 1 == 1).get()
            })?;

            // Subtract this bit_gadget from the linear combination to ensure the sums
            // balance out
            lc = lc - (coeff, b.get_variable());

            result_bits.push(b.into());

            max_value >>= 1;
            i += 1;
            coeff.double_in_place();
        }

        // Enforce that the linear combination equals zero
        cs.enforce(|| "modular subtraction", |lc| lc, |lc| lc, |_| lc);

        // Discard carry bits (probably unnecessary for subtraction
        result_bits.truncate(32);

        Ok(UInt32 {
            bits: result_bits,
            value: modular_value,
        })
    }

    /// Perform modular multiplication of two `UInt32` objects.
    pub fn mul<F: PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        // Make some arbitrary bounds for ourselves to avoid overflows
        // in the scalar field
        assert!(F::Params::MODULUS_BITS >= 64);

        // Compute the maximum value of the sum so we allocate enough bits for
        // the result
        let mut max_value = u64::from(u32::max_value());

        // Evaluate the self value
        let mut self_field_value = F::zero();
        let mut result_value = match self.value {
            Some(value) => {
                self_field_value = F::from(value as u128);
                Some(u64::from(value))
            }
            None => None,
        };

        // Evaluate the other value
        match other.value {
            Some(value) => {
                // Perform multiplication. If there is overflow this will fail.
                result_value.as_mut().map(|v| *v *= u64::from(value));
            }
            None => {
                result_value = None;
            }
        }

        let mut constant = true;

        // If any bits of self are allocated bits, return an allocated bit result
        for bit in &self.bits {
            match *bit {
                Boolean::Is(ref _bit) => constant = false,
                Boolean::Not(ref _bit) => constant = false,
                Boolean::Constant(_bit) => {}
            }
        }

        // This is a linear combination that we will enforce to be "zero"
        let mut lc = LinearCombination::zero();

        // Iterate over each bit_gadget of other and add the bit multiplied by the coefficient
        // to the linear combination
        let mut coeff = self_field_value;
        for bit in &other.bits {
            match *bit {
                Boolean::Is(ref bit) => {
                    constant = false;

                    // Add coeff * bit_gadget
                    lc = lc + (coeff, bit.get_variable());
                }
                Boolean::Not(ref bit) => {
                    constant = false;

                    // Add coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
                    lc = lc + (coeff, CS::one()) - (coeff, bit.get_variable());
                }
                Boolean::Constant(bit) => {
                    if bit {
                        lc = lc + (coeff, CS::one());
                    }
                }
            }

            coeff.double_in_place();
        }

        // The value of the actual result is moduluo 2^32
        let modular_value = result_value.map(|v| v as u32);

        if constant && modular_value.is_some() {
            // Return constant

            return Ok(UInt32::constant(modular_value.unwrap()));
        }

        // Storage for resulting bits
        let mut result_bits = vec![];

        // Allocate each bit gadget of the result
        let mut coeff = F::one();
        let mut i = 0;
        while max_value != 0 {
            // Allocate the bit_gadget
            let b = AllocatedBit::alloc(cs.ns(|| format!("multiplication result bit gadget {}", i)), || {
                result_value.map(|v| (v >> i) & 1 == 1).get()
            })?;

            // Subtract this bit_gadget from the linear combination to ensure the sums
            // balance out
            lc = lc - (coeff, b.get_variable());

            result_bits.push(b.into());

            max_value >>= 1;
            i += 1;
            coeff.double_in_place();
        }

        // Enforce that the linear combination equals zero
        cs.enforce(|| "modular multiplication", |lc| lc, |lc| lc, |_| lc);

        // Discard carry bits (probably unnecessary for multiplication
        result_bits.truncate(32);

        Ok(UInt32 {
            bits: result_bits,
            value: modular_value,
        })
    }

    /// Perform modular division of two `UInt32` objects.
    pub fn div<F: PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        // Make some arbitrary bounds for ourselves to avoid overflows
        // in the scalar field
        assert!(F::Params::MODULUS_BITS >= 64);

        // Compute the maximum value of the sum so we allocate enough bits for
        // the result
        let mut max_value = u64::from(u32::max_value());

        // Evaluate the self value
        let mut self_field_value = F::one();
        let mut result_value = match self.value {
            Some(value) => {
                self_field_value = F::from(value as u128);
                Some(u64::from(value))
            }
            None => None,
        };

        // Evaluate the other value
        let mut other_field_value = F::one();
        match other.value {
            Some(value) => {
                // Perform division. If there is overflow this will fail. Remainders are discarded.
                other_field_value = F::from(value as u128);
                result_value.as_mut().map(|v| *v /= u64::from(value));
            }
            None => {
                result_value = None;
            }
        }

        let mut constant = true;

        // If any bits of self are allocated, then we return an allocated bit result
        for bit in &self.bits {
            match *bit {
                Boolean::Is(ref _bit) => {
                    constant = false;
                }
                Boolean::Not(ref _bit) => {
                    constant = false;
                }
                Boolean::Constant(_bit) => {}
            }
        }

        // If any bits of other are allocated, then we return an allocated bit result
        for bit in &other.bits {
            match *bit {
                Boolean::Is(ref _bit) => {
                    constant = false;
                }
                Boolean::Not(ref _bit) => {
                    constant = false;
                }
                Boolean::Constant(_bit) => {}
            }
        }

        // This is a linear combination of the quotient that we will enforce to be "zero"
        let mut lc = LinearCombination::zero();

        // Perform bitwise long division, continually subtracting the divisor from the dividend
        // After each subtraction, add 1 to the lc quotient
        let mut dividend = self_field_value;
        while dividend.gt(&F::zero()) {
            lc = lc + (F::one(), CS::one());
            dividend -= &other_field_value;
        }
        //
        // // Iterate over each bit_gadget of self and subtract other from the linear combination
        // let mut coeff = other_field_value;
        // for bit in &self.bits {
        //     match *bit {
        //         Boolean::Is(ref bit) => {
        //             constant = false;
        //
        //             // Subtract coeff * bit_gadget
        //             lc = lc - (coeff, bit.get_variable());
        //         }
        //         Boolean::Not(ref bit) => {
        //             constant = false;
        //
        //             // Subtract coeff * (1 - bit_gadget) = coeff * ONE - coeff * bit_gadget
        //             lc = lc - (coeff, CS::one()) - (coeff, bit.get_variable());
        //         }
        //         Boolean::Constant(bit) => {
        //             if bit {
        //                 lc = lc - (coeff, CS::one());
        //             }
        //         }
        //     }
        //
        //     coeff.double_in_place();
        // }

        // The value of the actual result is moduluo 2^32
        let modular_value = result_value.map(|v| v as u32);

        if constant && modular_value.is_some() {
            // Return constant

            return Ok(UInt32::constant(modular_value.unwrap()));
        }

        // Storage for resulting bits
        let mut result_bits = vec![];

        // Allocate each bit gadget of the result
        let mut coeff = F::one();
        let mut i = 0;
        while max_value != 0 {
            // Allocate the bit_gadget
            let b = AllocatedBit::alloc(cs.ns(|| format!("division result bit gadget {}", i)), || {
                result_value.map(|v| (v >> i) & 1 == 1).get()
            })?;

            // Subtract this bit_gadget from the linear combination to ensure the sums
            // balance out
            lc = lc - (coeff, b.get_variable());

            result_bits.push(b.into());

            max_value >>= 1;
            i += 1;
            coeff.double_in_place();
        }

        // Enforce that the linear combination equals zero
        cs.enforce(|| "modular division", |lc| lc, |lc| lc, |_| lc);

        // Discard carry bits (probably unnecessary for multiplication
        result_bits.truncate(32);

        Ok(UInt32 {
            bits: result_bits,
            value: modular_value,
        })
    }

    /// Perform modular exponentiation of a `UInt32` object.
    pub fn pow<F: PrimeField, CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
    ) -> Result<Self, SynthesisError> {
        // Make some arbitrary bounds for ourselves to avoid overflows
        // in the scalar field
        assert!(F::Params::MODULUS_BITS >= 64);

        // Compute the maximum value of the sum so we allocate enough bits for
        // the result
        let mut max_value = u64::from(u32::max_value());

        // Evaluate the self value
        let mut self_field_value = F::one();
        let mut result_value = match self.value {
            Some(value) => {
                self_field_value = F::from(value as u128);
                Some(u64::from(value))
            }
            None => None,
        };

        // Evaluate the other value
        match other.value {
            Some(value) => {
                // Perform exponentiation. If there is overflow this will fail.
                result_value.as_mut().map(|v| *v = u64::from(*v).pow(value));
            }
            None => {
                result_value = None;
            }
        }

        let mut constant = true;

        // If any bits of self are allocated bits, return an allocated bit result
        for bit in &self.bits {
            match *bit {
                Boolean::Is(ref _bit) => constant = false,
                Boolean::Not(ref _bit) => constant = false,
                Boolean::Constant(_bit) => {}
            }
        }

        // This is a linear combination that we will enforce to be "zero"
        let mut lc = LinearCombination::zero();

        // Iterate over each bit_gadget of other and add the bit multiplied by the power
        // to the linear combination
        let mut power = self_field_value;
        for bit in &other.bits {
            match *bit {
                Boolean::Is(ref bit) => {
                    constant = false;

                    // Add power * bit_gadget
                    lc = lc + (power, bit.get_variable());
                }
                Boolean::Not(ref bit) => {
                    constant = false;

                    // Add power * (1 - other_lc) = power * ONE - power * other_lc
                    lc = lc + (power, CS::one()) - (power, bit.get_variable());
                }
                Boolean::Constant(bit) => {
                    if bit {
                        lc = lc + (power, CS::one());
                    }
                }
            }

            power.square_in_place();
        }

        // The value of the actual result is moduluo 2^32
        let modular_value = result_value.map(|v| v as u32);

        if constant && modular_value.is_some() {
            // Return constant

            return Ok(UInt32::constant(modular_value.unwrap()));
        }

        // Storage for resulting bits
        let mut result_bits = vec![];

        // Allocate each bit gadget of the result
        let mut coeff = F::one();
        let mut i = 0;
        while max_value != 0 {
            // Allocate the bit_gadget
            let b = AllocatedBit::alloc(cs.ns(|| format!("exponentiation result bit gadget {}", i)), || {
                result_value.map(|v| (v >> i) & 1 == 1).get()
            })?;

            // Subtract this bit_gadget from the linear combination to ensure the sums
            // balance out
            lc = lc - (coeff, b.get_variable());

            result_bits.push(b.into());

            max_value >>= 1;
            i += 1;
            coeff.double_in_place();
        }

        // Enforce that the linear combination equals zero
        cs.enforce(|| "modular exponentiation", |lc| lc, |lc| lc, |_| lc);

        // Discard carry bits (probably unnecessary for multiplication
        result_bits.truncate(32);

        Ok(UInt32 {
            bits: result_bits,
            value: modular_value,
        })
    }
}

impl<F: Field> ToBytesGadget<F> for UInt32 {
    #[inline]
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let value_chunks = match self.value.map(|val| {
            let mut bytes = [0u8; 4];
            val.write(bytes.as_mut()).unwrap();
            bytes
        }) {
            Some(chunks) => [Some(chunks[0]), Some(chunks[1]), Some(chunks[2]), Some(chunks[3])],
            None => [None, None, None, None],
        };
        let mut bytes = Vec::new();
        for (i, chunk8) in self.to_bits_le().chunks(8).into_iter().enumerate() {
            let byte = UInt8 {
                bits: chunk8.to_vec(),
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

impl PartialEq for UInt32 {
    fn eq(&self, other: &Self) -> bool {
        !self.value.is_none() && !other.value.is_none() && self.value == other.value
    }
}

impl Eq for UInt32 {}

impl<F: Field> ConditionalEqGadget<F> for UInt32 {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (a, b)) in self.bits.iter().zip(&other.bits).enumerate() {
            a.conditional_enforce_equal(&mut cs.ns(|| format!("uint32_equal_{}", i)), b, condition)?;
        }
        Ok(())
    }

    fn cost() -> usize {
        32 * <Boolean as ConditionalEqGadget<F>>::cost()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        curves::Field,
        gadgets::{
            r1cs::{Fr, TestConstraintSystem},
            utilities::boolean::Boolean,
        },
    };

    use rand::{Rng, SeedableRng};
    use rand_xorshift::XorShiftRng;

    #[test]
    fn test_uint32_from_bits() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let v = (0..32).map(|_| Boolean::constant(rng.gen())).collect::<Vec<_>>();

            let b = UInt32::from_bits_le(&v);

            for (i, bit_gadget) in b.bits.iter().enumerate() {
                match bit_gadget {
                    &Boolean::Constant(bit_gadget) => {
                        assert!(bit_gadget == ((b.value.unwrap() >> i) & 1 == 1));
                    }
                    _ => unreachable!(),
                }
            }

            let expected_to_be_same = b.to_bits_le();

            for x in v.iter().zip(expected_to_be_same.iter()) {
                match x {
                    (&Boolean::Constant(true), &Boolean::Constant(true)) => {}
                    (&Boolean::Constant(false), &Boolean::Constant(false)) => {}
                    _ => unreachable!(),
                }
            }
        }
    }

    #[test]
    fn test_uint32_xor() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let mut cs = TestConstraintSystem::<Fr>::new();

            let a: u32 = rng.gen();
            let b: u32 = rng.gen();
            let c: u32 = rng.gen();

            let mut expected = a ^ b ^ c;

            let a_bit = UInt32::alloc(cs.ns(|| "a_bit"), Some(a)).unwrap();
            let b_bit = UInt32::constant(b);
            let c_bit = UInt32::alloc(cs.ns(|| "c_bit"), Some(c)).unwrap();

            let r = a_bit.xor(cs.ns(|| "first xor"), &b_bit).unwrap();
            let r = r.xor(cs.ns(|| "second xor"), &c_bit).unwrap();

            assert!(cs.is_satisfied());

            assert!(r.value == Some(expected));

            for b in r.bits.iter() {
                match b {
                    &Boolean::Is(ref b) => {
                        assert!(b.get_value().unwrap() == (expected & 1 == 1));
                    }
                    &Boolean::Not(ref b) => {
                        assert!(!b.get_value().unwrap() == (expected & 1 == 1));
                    }
                    &Boolean::Constant(b) => {
                        assert!(b == (expected & 1 == 1));
                    }
                }

                expected >>= 1;
            }
        }
    }

    #[test]
    fn test_uint32_addmany_constants() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let mut cs = TestConstraintSystem::<Fr>::new();

            let a: u32 = rng.gen();
            let b: u32 = rng.gen();
            let c: u32 = rng.gen();

            let a_bit = UInt32::constant(a);
            let b_bit = UInt32::constant(b);
            let c_bit = UInt32::constant(c);

            let mut expected = a.wrapping_add(b).wrapping_add(c);

            let r = UInt32::addmany(cs.ns(|| "addition"), &[a_bit, b_bit, c_bit]).unwrap();

            assert!(r.value == Some(expected));

            for b in r.bits.iter() {
                match b {
                    &Boolean::Is(_) => panic!(),
                    &Boolean::Not(_) => panic!(),
                    &Boolean::Constant(b) => {
                        assert!(b == (expected & 1 == 1));
                    }
                }

                expected >>= 1;
            }
        }
    }

    #[test]
    fn test_uint32_addmany() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let mut cs = TestConstraintSystem::<Fr>::new();

            let a: u32 = rng.gen();
            let b: u32 = rng.gen();
            let c: u32 = rng.gen();
            let d: u32 = rng.gen();

            let mut expected = (a ^ b).wrapping_add(c).wrapping_add(d);

            let a_bit = UInt32::alloc(cs.ns(|| "a_bit"), Some(a)).unwrap();
            let b_bit = UInt32::constant(b);
            let c_bit = UInt32::constant(c);
            let d_bit = UInt32::alloc(cs.ns(|| "d_bit"), Some(d)).unwrap();

            let r = a_bit.xor(cs.ns(|| "xor"), &b_bit).unwrap();
            let r = UInt32::addmany(cs.ns(|| "addition"), &[r, c_bit, d_bit]).unwrap();

            assert!(cs.is_satisfied());

            assert!(r.value == Some(expected));

            for b in r.bits.iter() {
                match b {
                    &Boolean::Is(ref b) => {
                        assert!(b.get_value().unwrap() == (expected & 1 == 1));
                    }
                    &Boolean::Not(ref b) => {
                        assert!(!b.get_value().unwrap() == (expected & 1 == 1));
                    }
                    &Boolean::Constant(_) => unreachable!(),
                }

                expected >>= 1;
            }

            // Flip a bit_gadget and see if the addition constraint still works
            if cs.get("addition/result bit_gadget 0/boolean").is_zero() {
                cs.set("addition/result bit_gadget 0/boolean", Field::one());
            } else {
                cs.set("addition/result bit_gadget 0/boolean", Field::zero());
            }

            assert!(!cs.is_satisfied());
        }
    }

    #[test]
    fn test_uint32_rotr() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        let mut num = rng.gen();

        let a = UInt32::constant(num);

        for i in 0..32 {
            let b = a.rotr(i);

            assert!(b.value.unwrap() == num);

            let mut tmp = num;
            for b in &b.bits {
                match b {
                    &Boolean::Constant(b) => {
                        assert_eq!(b, tmp & 1 == 1);
                    }
                    _ => unreachable!(),
                }

                tmp >>= 1;
            }

            num = num.rotate_right(1);
        }
    }

    #[test]
    fn test_sub() {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let x = UInt32::constant(100u32);
        let y = UInt32::alloc(cs.ns(|| format!("alloc")), Some(99)).unwrap();

        let res = x.sub(cs.ns(|| "sub".to_string()), &y).unwrap();

        println!("{:?}", res.value.unwrap());
        println!("{:?}", res.bits.to_vec());
        println!("{:?}", cs.num_constraints());
        assert!(cs.is_satisfied());
    }

    #[test]
    fn test_mul() {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let x = UInt32::constant(9u32);
        // let y = UInt32::constant(9u32);
        let z = UInt32::alloc(cs.ns(|| format!("alloc")), Some(9u32)).unwrap();

        let res = x.mul(cs.ns(|| "mul".to_string()), &z).unwrap();

        println!("{:?}", res.value.unwrap());
        // println!("{:?}", res.bits.to_vec());
        println!("{:?}", cs.num_constraints());
        assert!(cs.is_satisfied());
    }

    #[test]
    fn test_div() {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let x = UInt32::constant(64u32);
        // let y = UInt32::constant(2);
        let z = UInt32::alloc(cs.ns(|| format!("alloc")), Some(4u32)).unwrap();

        let res = x.div(cs.ns(|| "div".to_string()), &z).unwrap();

        println!("{:?}", res.value.unwrap());
        println!("{:?}", res.bits.to_vec());
        println!("{:?}", cs.num_constraints());
        assert!(cs.is_satisfied()); //This fails
    }

    #[test]
    fn test_pow() {
        let mut cs = TestConstraintSystem::<Fr>::new();

        let x = UInt32::constant(2);
        // let y = UInt32::constant(2);
        let z = UInt32::alloc(cs.ns(|| format!("alloc")), Some(8)).unwrap();

        let res = x.pow(cs.ns(|| format!("pow")), &z).unwrap();

        println!("{:?}", res.value.unwrap());
        println!("{:?}", res.bits.to_vec());
        println!("{:?}", cs.num_constraints());
        assert!(cs.is_satisfied());
    }

    // #[test]
    // fn test_add() {
    //     let mut cs = TestConstraintSystem::<Fr>::new();
    //
    //     let five = UInt32::constant(5u32);
    //     let three = UInt32::alloc(cs.ns(||format!("alloc")), Some(3u32)).unwrap();
    //
    //     let result = five.add(cs.ns(||format!("add")), &three).unwrap();
    //
    //     let eight = UInt32::alloc(cs.ns(||format!("eight")), Some(8u32)).unwrap();
    //
    //     println!("result {:#?}", result.bits);
    //     println!("eight {:#?}", eight.bits);
    //
    //     // assert_eq!(result.bits, eight.bits);
    // }
}
