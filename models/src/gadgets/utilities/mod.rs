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
    curves::Field,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            boolean::Boolean,
            uint::unsigned_integer::{UInt, UInt8},
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

pub mod alloc;
pub mod arithmetic;
pub mod bits;
pub mod boolean;
pub mod eq;
pub mod int;
pub mod select;
pub mod uint;

pub trait ToBitsGadget<F: Field> {
    fn to_bits<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError>;

    /// Additionally checks if the produced list of booleans is 'valid'.
    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError>;
}

impl<F: Field> ToBitsGadget<F> for Boolean {
    fn to_bits<CS: ConstraintSystem<F>>(&self, _: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(vec![*self])
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, _: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(vec![*self])
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
            result.extend_from_slice(&byte.to_bits_le());
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

impl<F: Field> ToBytesGadget<F> for Vec<UInt8> {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, _cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.to_vec())
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}
