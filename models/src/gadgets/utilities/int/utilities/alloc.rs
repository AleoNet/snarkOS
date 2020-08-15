use crate::{
    curves::{to_field_vec::ToConstraintField, Field, FpParameters, PrimeField},
    gadgets::{
        curves::FpGadget,
        r1cs::ConstraintSystem,
        utilities::{
            alloc::AllocGadget,
            boolean::{AllocatedBit, Boolean},
            eq::EqGadget,
            int::{Int, Int64},
            ToBitsGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use core::borrow::Borrow;

macro_rules! alloc_int_impl {
    ($($gadget: ident)*) => ($(
        impl<F: Field> AllocGadget<<$gadget as Int>::IntegerType, F> for $gadget {
            fn alloc<
                Fn: FnOnce() -> Result<T, SynthesisError>,
                T: Borrow<<$gadget as Int>::IntegerType>,
                CS: ConstraintSystem<F>
            >(
                mut cs: CS,
                value_gen: Fn,
            ) -> Result<Self, SynthesisError> {
                let value = value_gen().map(|val| *val.borrow());
                let values = match value {
                    Ok(mut val) => {
                        let mut v = Vec::with_capacity(<$gadget as Int>::SIZE);

                        for _ in 0..<$gadget as Int>::SIZE {
                            v.push(Some(val & 1 == 1));
                            val >>= 1;
                        }

                        v
                    }
                    _ => vec![None; <$gadget as Int>::SIZE],
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

            fn alloc_input<
                Fn: FnOnce() -> Result<T, SynthesisError>,
                T: Borrow<<$gadget as Int>::IntegerType>,
                CS: ConstraintSystem<F>
            >(
                mut cs: CS,
                value_gen: Fn,
            ) -> Result<Self, SynthesisError> {
                let value = value_gen().map(|val| *val.borrow());
                let values = match value {
                    Ok(mut val) => {
                        let mut v = Vec::with_capacity(<$gadget as Int>::SIZE);

                        for _ in 0..<$gadget as Int>::SIZE {
                            v.push(Some(val & 1 == 1));
                            val >>= 1;
                        }

                        v
                    }
                    _ => vec![None; <$gadget as Int>::SIZE],
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
    )*)
}

alloc_int_impl!(Int64);

/// Alloc the unsigned integer through field elements rather purely bits
/// to reduce the number of input allocations.
macro_rules! alloc_input_fe {
    ($($gadget: ident)*) => ($(
        impl $gadget {
            /// Allocates the unsigned integer gadget by first converting
            /// the little-endian byte representation of the unsigned integer to
            /// `F` elements, (thus reducing the number of input allocations),
            /// and then converts this list of `F` gadgets into the unsigned integer gadget
            pub fn alloc_input_fe<F, CS>(mut cs: CS, value: <$gadget as Int>::IntegerType) -> Result<Self, SynthesisError>
            where
                F: PrimeField,
                CS: ConstraintSystem<F>,
            {
                let value_bytes = value.to_le_bytes();
                let field_elements: Vec<F> = ToConstraintField::<F>::to_field_elements(&value_bytes[..]).unwrap();

                let max_size = 8 * (F::Parameters::CAPACITY / 8) as usize;
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

                // Assert that the extra bits are false
                for (i, bit) in allocated_bits.iter().skip(<$gadget as Int>::SIZE).enumerate() {
                    bit.enforce_equal(&mut cs.ns(|| format!("bit {} is false", i + <$gadget as Int>::SIZE)), &Boolean::constant(false))?;
                }

                let bits = allocated_bits[0..<$gadget as Int>::SIZE].to_vec();

                Ok(Self {
                    bits,
                    value: Some(value),
                })
            }
        }
    )*)
}

alloc_input_fe!(Int64);
