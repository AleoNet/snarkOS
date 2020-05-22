use crate::{
    curves::{to_field_vec::ToConstraintField, Field, FpParameters, PrimeField},
    gadgets::{
        curves::fp::FpGadget,
        r1cs::{Assignment, ConstraintSystem},
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            eq::{ConditionalEqGadget, EqGadget},
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::borrow::Borrow;

/// Represents an interpretation of 8 `Boolean` objects as an
/// unsigned integer.
#[derive(Clone, Debug)]
pub struct UInt8 {
    // Least significant bit_gadget first
    pub(crate) bits: Vec<Boolean>,
    pub(crate) value: Option<u8>,
}

impl UInt8 {
    pub fn get_value(&self) -> Option<u8> {
        self.value
    }

    /// Construct a constant vector of `UInt8` from a vector of `u8`
    pub fn constant_vec(values: &[u8]) -> Vec<Self> {
        let mut result = Vec::new();
        for value in values {
            result.push(UInt8::constant(*value));
        }
        result
    }

    /// Construct a constant `UInt8` from a `u8`
    pub fn constant(value: u8) -> Self {
        let mut bits = Vec::with_capacity(8);

        let mut tmp = value;
        for _ in 0..8 {
            // If last bit is one, push one.
            if tmp & 1 == 1 {
                bits.push(Boolean::constant(true))
            } else {
                bits.push(Boolean::constant(false))
            }

            tmp >>= 1;
        }

        Self {
            bits,
            value: Some(value),
        }
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

    /// Turns this `UInt8` into its little-endian byte order representation.
    /// LSB-first means that we can easily get the corresponding field element
    /// via double and add.
    pub fn into_bits_le(&self) -> Vec<Boolean> {
        self.bits.iter().cloned().collect()
    }

    /// Converts a little-endian byte order representation of bits into a
    /// `UInt8`.
    pub fn from_bits_le(bits: &[Boolean]) -> Self {
        assert_eq!(bits.len(), 8);

        let bits = bits.to_vec();

        let mut value = Some(0u8);
        for b in bits.iter().rev() {
            value.as_mut().map(|v| *v <<= 1);

            match *b {
                Boolean::Constant(b) => {
                    if b {
                        value.as_mut().map(|v| *v |= 1);
                    }
                }
                Boolean::Is(ref b) => match b.get_value() {
                    Some(true) => {
                        value.as_mut().map(|v| *v |= 1);
                    }
                    Some(false) => {}
                    None => value = None,
                },
                Boolean::Not(ref b) => match b.get_value() {
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

    /// XOR this `UInt8` with another `UInt8`
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

        Ok(Self { bits, value: new_value })
    }
}

impl PartialEq for UInt8 {
    fn eq(&self, other: &Self) -> bool {
        !self.value.is_none() && !other.value.is_none() && self.value == other.value
    }
}

impl Eq for UInt8 {}

impl<F: Field> ConditionalEqGadget<F> for UInt8 {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        for (i, (a, b)) in self.bits.iter().zip(&other.bits).enumerate() {
            a.conditional_enforce_equal(
                &mut cs.ns(|| format!("UInt8 equality check for {}-th bit", i)),
                b,
                condition,
            )?;
        }
        Ok(())
    }

    fn cost() -> usize {
        8 * <Boolean as ConditionalEqGadget<F>>::cost()
    }
}

impl<F: Field> EqGadget<F> for UInt8 {}

impl<F: Field> AllocGadget<u8, F> for UInt8 {
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<u8>,
    {
        let value = value_gen().map(|val| *val.borrow());
        let values = match value {
            Ok(mut val) => {
                let mut v = Vec::with_capacity(8);

                for _ in 0..8 {
                    v.push(Some(val & 1 == 1));
                    val >>= 1;
                }

                v
            }
            _ => vec![None; 8],
        };

        let bits = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                Ok(Boolean::from(AllocatedBit::alloc(
                    &mut cs.ns(|| format!("allocated bit_gadget {}", i)),
                    || v.ok_or(SynthesisError::AssignmentMissing),
                )?))
            })
            .collect::<Result<Vec<_>, SynthesisError>>()?;

        Ok(Self {
            bits,
            value: value.ok(),
        })
    }

    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<u8>,
    {
        let value = value_gen().map(|val| *val.borrow());
        let values = match value {
            Ok(mut val) => {
                let mut v = Vec::with_capacity(8);
                for _ in 0..8 {
                    v.push(Some(val & 1 == 1));
                    val >>= 1;
                }

                v
            }
            _ => vec![None; 8],
        };

        let bits = values
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                Ok(Boolean::from(AllocatedBit::alloc_input(
                    &mut cs.ns(|| format!("allocated bit_gadget {}", i)),
                    || v.ok_or(SynthesisError::AssignmentMissing),
                )?))
            })
            .collect::<Result<Vec<_>, SynthesisError>>()?;

        Ok(Self {
            bits,
            value: value.ok(),
        })
    }
}

impl<F: Field> ToBytesGadget<F> for Vec<UInt8> {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.to_vec())
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::gadgets::r1cs::{Fr, TestConstraintSystem};

    use rand::{Rng, SeedableRng};
    use rand_xorshift::XorShiftRng;

    #[test]
    fn test_uint8_from_bits_to_bits() {
        let mut cs = TestConstraintSystem::<Fr>::new();
        let byte_val = 0b01110001;
        let byte = UInt8::alloc(cs.ns(|| "alloc value"), || Ok(byte_val)).unwrap();
        let bits = byte.into_bits_le();
        for (i, bit) in bits.iter().enumerate() {
            assert_eq!(bit.get_value().unwrap(), (byte_val >> i) & 1 == 1)
        }
    }

    #[test]
    fn test_uint8_alloc_input_vec() {
        let mut cs = TestConstraintSystem::<Fr>::new();
        let byte_vals = (64u8..128u8).into_iter().collect::<Vec<_>>();
        let bytes = UInt8::alloc_input_vec(cs.ns(|| "alloc value"), &byte_vals).unwrap();
        for (native_byte, gadget_byte) in byte_vals.into_iter().zip(bytes) {
            let bits = gadget_byte.into_bits_le();
            for (i, bit) in bits.iter().enumerate() {
                assert_eq!(bit.get_value().unwrap(), (native_byte >> i) & 1 == 1)
            }
        }
    }

    #[test]
    fn test_uint8_from_bits() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let v = (0..8).map(|_| Boolean::constant(rng.gen())).collect::<Vec<_>>();

            let b = UInt8::from_bits_le(&v);

            for (i, bit_gadget) in b.bits.iter().enumerate() {
                match bit_gadget {
                    &Boolean::Constant(bit_gadget) => {
                        assert!(bit_gadget == ((b.value.unwrap() >> i) & 1 == 1));
                    }
                    _ => unreachable!(),
                }
            }

            let expected_to_be_same = b.into_bits_le();

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
    fn test_uint8_xor() {
        let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

        for _ in 0..1000 {
            let mut cs = TestConstraintSystem::<Fr>::new();

            let a: u8 = rng.gen();
            let b: u8 = rng.gen();
            let c: u8 = rng.gen();

            let mut expected = a ^ b ^ c;

            let a_bit = UInt8::alloc(cs.ns(|| "a_bit"), || Ok(a)).unwrap();
            let b_bit = UInt8::constant(b);
            let c_bit = UInt8::alloc(cs.ns(|| "c_bit"), || Ok(c)).unwrap();

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
}
