use crate::{
    curves::PrimeField,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{arithmetic::Neg, int::*},
    },
};
use snarkos_errors::gadgets::SignedIntegerError;

macro_rules! neg_int_impl {
    ($($gadget: ident)*) => ($(
        impl<F: PrimeField> Neg<F> for $gadget {
            type ErrorType = SignedIntegerError;

            fn neg<CS: ConstraintSystem<F>>(
                &self,
                cs: CS
            ) -> Result<Self, Self::ErrorType> {
                let value = match self.value {
                    Some(val) => {
                        match val.checked_neg() {
                            Some(val_neg) => Some(val_neg),
                            None => return Err(SignedIntegerError::Overflow) // -0 should fail
                        }
                    }
                    None => None,
                };

                // calculate two's complement
                let bits = self.bits.neg(cs)?;

                Ok(Self {
                    bits,
                    value,
                })
            }
        }
    )*)
}

neg_int_impl!(Int64);
