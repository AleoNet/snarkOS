use crate::{
    curves::Field,
    gadgets::{
        r1cs::{ConstraintSystem, SynthesisError},
        utilities::{boolean::Boolean, uint8::UInt8},
    },
};

pub mod alloc;
pub mod boolean;
pub mod eq;
pub mod select;
pub mod uint32;
pub mod uint8;

pub trait ToBitsGadget<F: Field> {
    fn to_bits<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError>;

    /// Additionally checks if the produced list of booleans is 'valid'.
    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError>;
}

impl<F: Field> ToBitsGadget<F> for Boolean {
    fn to_bits<CS: ConstraintSystem<F>>(&self, _: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(vec![self.clone()])
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, _: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(vec![self.clone()])
    }
}

impl<F: Field> ToBitsGadget<F> for [Boolean] {
    fn to_bits<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(self.to_vec())
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(self.to_vec())
    }
}
impl<F: Field> ToBitsGadget<F> for Vec<Boolean> {
    fn to_bits<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(self.clone())
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(self.clone())
    }
}

impl<F: Field> ToBitsGadget<F> for [UInt8] {
    fn to_bits<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut result = Vec::with_capacity(&self.len() * 8);
        for byte in self {
            result.extend_from_slice(&byte.into_bits_le());
        }
        Ok(result)
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        self.to_bits(cs)
    }
}

pub trait ToBytesGadget<F: Field> {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError>;

    /// Additionally checks if the produced list of booleans is 'valid'.
    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError>;
}

impl<F: Field> ToBytesGadget<F> for [UInt8] {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.to_vec())
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

impl<'a, F: Field, T: 'a + ToBytesGadget<F>> ToBytesGadget<F> for &'a T {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        (*self).to_bytes(cs)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

impl<'a, F: Field> ToBytesGadget<F> for &'a [UInt8] {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.to_vec())
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}
