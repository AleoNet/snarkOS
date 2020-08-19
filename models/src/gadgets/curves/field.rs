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
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget, NEqGadget},
            select::{CondSelectGadget, ThreeBitCondNegLookupGadget, TwoBitLookupGadget},
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::fmt::Debug;

pub trait FieldGadget<NativeF: Field, F: Field>:
    Sized
    + Clone
    + EqGadget<F>
    + NEqGadget<F>
    + ConditionalEqGadget<F>
    + ToBitsGadget<F>
    + AllocGadget<NativeF, F>
    + ToBytesGadget<F>
    + CondSelectGadget<F>
    + TwoBitLookupGadget<F, TableConstant = NativeF>
    + ThreeBitCondNegLookupGadget<F, TableConstant = NativeF>
    + Debug
{
    type Variable: Clone + Debug;

    fn get_value(&self) -> Option<NativeF>;

    fn get_variable(&self) -> Self::Variable;

    fn zero<CS: ConstraintSystem<F>>(_: CS) -> Result<Self, SynthesisError>;

    fn one<CS: ConstraintSystem<F>>(_: CS) -> Result<Self, SynthesisError>;

    fn conditionally_add_constant<CS: ConstraintSystem<F>>(
        &self,
        _: CS,
        _: &Boolean,
        _: NativeF,
    ) -> Result<Self, SynthesisError>;

    fn add<CS: ConstraintSystem<F>>(&self, _: CS, _: &Self) -> Result<Self, SynthesisError>;

    fn add_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS, other: &Self) -> Result<&mut Self, SynthesisError> {
        *self = self.add(cs, other)?;
        Ok(self)
    }

    fn double<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Self, SynthesisError> {
        self.add(cs, &self)
    }

    fn double_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS) -> Result<&mut Self, SynthesisError> {
        *self = self.double(cs)?;
        Ok(self)
    }

    fn sub<CS: ConstraintSystem<F>>(&self, _: CS, _: &Self) -> Result<Self, SynthesisError>;

    fn sub_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS, other: &Self) -> Result<&mut Self, SynthesisError> {
        *self = self.sub(cs, other)?;
        Ok(self)
    }

    fn negate<CS: ConstraintSystem<F>>(&self, _: CS) -> Result<Self, SynthesisError>;

    #[inline]
    fn negate_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS) -> Result<&mut Self, SynthesisError> {
        *self = self.negate(cs)?;
        Ok(self)
    }

    fn mul<CS: ConstraintSystem<F>>(&self, _: CS, _: &Self) -> Result<Self, SynthesisError>;

    fn mul_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS, other: &Self) -> Result<&mut Self, SynthesisError> {
        *self = self.mul(cs, other)?;
        Ok(self)
    }

    fn square<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Self, SynthesisError> {
        self.mul(cs, &self)
    }

    fn square_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS) -> Result<&mut Self, SynthesisError> {
        *self = self.square(cs)?;
        Ok(self)
    }

    fn mul_equals<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        result: &Self,
    ) -> Result<(), SynthesisError> {
        let actual_result = self.mul(cs.ns(|| "calc_actual_result"), other)?;
        result.enforce_equal(&mut cs.ns(|| "test_equals"), &actual_result)
    }

    fn square_equals<CS: ConstraintSystem<F>>(&self, mut cs: CS, result: &Self) -> Result<(), SynthesisError> {
        let actual_result = self.square(cs.ns(|| "calc_actual_result"))?;
        result.enforce_equal(&mut cs.ns(|| "test_equals"), &actual_result)
    }

    fn add_constant<CS: ConstraintSystem<F>>(&self, _: CS, _: &NativeF) -> Result<Self, SynthesisError>;

    fn add_constant_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        cs: CS,
        other: &NativeF,
    ) -> Result<&mut Self, SynthesisError> {
        *self = self.add_constant(cs, other)?;
        Ok(self)
    }

    fn sub_constant<CS: ConstraintSystem<F>>(&self, cs: CS, fe: &NativeF) -> Result<Self, SynthesisError> {
        self.add_constant(cs, &(-(*fe)))
    }

    fn sub_constant_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        cs: CS,
        other: &NativeF,
    ) -> Result<&mut Self, SynthesisError> {
        self.add_constant_in_place(cs, &(-(*other)))
    }

    fn mul_by_constant<CS: ConstraintSystem<F>>(&self, _: CS, _: &NativeF) -> Result<Self, SynthesisError>;

    fn mul_by_constant_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        cs: CS,
        other: &NativeF,
    ) -> Result<&mut Self, SynthesisError> {
        *self = self.mul_by_constant(cs, other)?;
        Ok(self)
    }

    fn inverse<CS: ConstraintSystem<F>>(&self, _: CS) -> Result<Self, SynthesisError>;

    fn frobenius_map<CS: ConstraintSystem<F>>(&self, _: CS, power: usize) -> Result<Self, SynthesisError>;

    fn frobenius_map_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        cs: CS,
        power: usize,
    ) -> Result<&mut Self, SynthesisError> {
        *self = self.frobenius_map(cs, power)?;
        Ok(self)
    }

    /// Accepts as input a list of bits which, when interpreted in big-endian
    /// form, are a scalar.
    #[inline]
    fn pow<CS: ConstraintSystem<F>>(&self, mut cs: CS, bits: &[Boolean]) -> Result<Self, SynthesisError> {
        let mut res = Self::one(cs.ns(|| "Alloc result"))?;
        for (i, bit) in bits.into_iter().enumerate() {
            res = res.square(cs.ns(|| format!("Double {}", i)))?;
            let tmp = res.mul(cs.ns(|| format!("Add {}-th base power", i)), self)?;
            res = Self::conditionally_select(cs.ns(|| format!("Conditional Select {}", i)), bit, &tmp, &res)?;
        }
        Ok(res)
    }

    fn cost_of_mul() -> usize;

    fn cost_of_inv() -> usize;
}
