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

use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{ModelParameters, MontgomeryModelParameters, PrimeField},
    gadgets::{
        curves::FpGadget,
        r1cs::ConstraintSystem,
        utilities::{alloc::AllocGadget, boolean::Boolean, uint::UInt8, ToBitsGadget, ToBytesGadget},
    },
};
use snarkos_utilities::{to_bytes, ToBytes};

use std::{borrow::Borrow, marker::PhantomData};

#[derive(Clone, Debug)]
pub struct Elligator2FieldGadget<P: MontgomeryModelParameters, F: PrimeField>(pub FpGadget<F>, PhantomData<P>);

impl<P: MontgomeryModelParameters, F: PrimeField> AllocGadget<<P as ModelParameters>::BaseField, F>
    for Elligator2FieldGadget<P, F>
{
    fn alloc<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<<P as ModelParameters>::BaseField>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Elligator2FieldGadget(
            FpGadget::alloc(cs, || match value_gen() {
                Ok(value) => Ok(F::read(&to_bytes![value.borrow()]?[..])?),
                Err(_) => Err(SynthesisError::AssignmentMissing),
            })?,
            PhantomData,
        ))
    }

    fn alloc_input<
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<<P as ModelParameters>::BaseField>,
        CS: ConstraintSystem<F>,
    >(
        cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        Ok(Elligator2FieldGadget(
            FpGadget::alloc_input(cs, || match value_gen() {
                Ok(value) => Ok(F::read(&to_bytes![value.borrow()]?[..])?),
                Err(_) => Err(SynthesisError::AssignmentMissing),
            })?,
            PhantomData,
        ))
    }
}

impl<P: MontgomeryModelParameters, F: PrimeField> ToBitsGadget<F> for Elligator2FieldGadget<P, F> {
    fn to_bits<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(self.0.to_bits(cs)?)
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        Ok(self.0.to_bits_strict(cs)?)
    }
}

impl<P: MontgomeryModelParameters, F: PrimeField> ToBytesGadget<F> for Elligator2FieldGadget<P, F> {
    fn to_bytes<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.0.to_bytes(cs)?)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        Ok(self.0.to_bytes_strict(cs)?)
    }
}
