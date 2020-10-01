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
    curves::{
        batch_inversion,
        fp6_3over2::{Fp6, Fp6Parameters},
        Field,
        Fp2Parameters,
        PrimeField,
    },
    gadgets::{
        curves::FieldGadget,
        r1cs::{Assignment, ConstraintSystem, ConstraintVar},
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget, NEqGadget},
            select::{CondSelectGadget, ThreeBitCondNegLookupGadget, TwoBitLookupGadget},
            uint::UInt8,
            ToBitsGadget,
            ToBytesGadget,
        },
    },
};
use snarkos_errors::gadgets::SynthesisError;

use std::{borrow::Borrow, marker::PhantomData};

type Fp2Gadget<P, F> = super::fp2::Fp2Gadget<<P as Fp6Parameters>::Fp2Params, F>;

#[derive(Derivative)]
#[derivative(Debug(bound = "F: PrimeField"))]
#[must_use]
pub struct Fp6Gadget<P, F: PrimeField>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    pub c0: Fp2Gadget<P, F>,
    pub c1: Fp2Gadget<P, F>,
    pub c2: Fp2Gadget<P, F>,
    #[derivative(Debug = "ignore")]
    _params: PhantomData<P>,
}

impl<P, F: PrimeField> Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    pub fn new(c0: Fp2Gadget<P, F>, c1: Fp2Gadget<P, F>, c2: Fp2Gadget<P, F>) -> Self {
        Self {
            c0,
            c1,
            c2,
            _params: PhantomData,
        }
    }

    /// Multiply a Fp2Gadget by cubic nonresidue P::NONRESIDUE.
    #[inline]
    pub fn mul_fp2_gadget_by_nonresidue<CS: ConstraintSystem<F>>(
        cs: CS,
        fe: &Fp2Gadget<P, F>,
    ) -> Result<Fp2Gadget<P, F>, SynthesisError> {
        fe.mul_by_constant(cs, &P::NONRESIDUE)
    }

    #[inline]
    pub fn mul_by_0_c1_0<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        c1: &Fp2Gadget<P, F>,
    ) -> Result<Self, SynthesisError> {
        // Karatsuba multiplication
        // v0 = a0 * b0 = 0

        // v1 = a1 * b1
        let v1 = self.c1.mul(cs.ns(|| "first mul"), c1)?;

        // v2 = a2 * b2 = 0

        let a1_plus_a2 = self.c1.add(cs.ns(|| "a1 + a2"), &self.c2)?;
        let b1_plus_b2 = c1.clone();

        let a0_plus_a1 = self.c0.add(cs.ns(|| "a0 + a1"), &self.c1)?;

        // c0 = (NONRESIDUE * ((a1 + a2)*(b1 + b2) - v1 - v2)) + v0
        //    = NONRESIDUE * ((a1 + a2) * b1 - v1)
        let c0 = a1_plus_a2
            .mul(cs.ns(|| "second mul"), &b1_plus_b2)?
            .sub(cs.ns(|| "first sub"), &v1)?
            .mul_by_constant(cs.ns(|| "mul_by_nonresidue"), &P::NONRESIDUE)?;

        // c1 = (a0 + a1) * (b0 + b1) - v0 - v1 + NONRESIDUE * v2
        //    = (a0 + a1) * b1 - v1
        let c1 = a0_plus_a1
            .mul(cs.ns(|| "third mul"), &c1)?
            .sub(cs.ns(|| "second sub"), &v1)?;
        // c2 = (a0 + a2) * (b0 + b2) - v0 - v2 + v1
        //    = v1
        let c2 = v1;
        Ok(Self::new(c0, c1, c2))
    }

    // #[inline]
    pub fn mul_by_c0_c1_0<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        c0: &Fp2Gadget<P, F>,
        c1: &Fp2Gadget<P, F>,
    ) -> Result<Self, SynthesisError> {
        let v0 = self.c0.mul(cs.ns(|| "v0"), c0)?;
        let v1 = self.c1.mul(cs.ns(|| "v1"), c1)?;
        // v2 = 0.

        let a1_plus_a2 = self.c1.add(cs.ns(|| "a1 + a2"), &self.c2)?;
        let a0_plus_a1 = self.c0.add(cs.ns(|| "a0 + a1"), &self.c1)?;
        let a0_plus_a2 = self.c0.add(cs.ns(|| "a0 + a2"), &self.c2)?;

        let b1_plus_b2 = c1.clone();
        let b0_plus_b1 = c0.add(cs.ns(|| "b0 + b1"), &c1)?;
        let b0_plus_b2 = c0.clone();

        let c0 = {
            let cs = &mut cs.ns(|| "c0");
            a1_plus_a2
                .mul(cs.ns(|| "(a1 + a2) * (b1 + b2)"), &b1_plus_b2)?
                .sub(cs.ns(|| "sub v1"), &v1)?
                .mul_by_constant(cs.ns(|| "First mul_by_nonresidue"), &P::NONRESIDUE)?
                .add(cs.ns(|| "add v0"), &v0)?
        };

        let c1 = {
            let cs = &mut cs.ns(|| "c1");
            a0_plus_a1
                .mul(cs.ns(|| "(a0 + a1) * (b0 + b1)"), &b0_plus_b1)?
                .sub(cs.ns(|| "sub v0"), &v0)?
                .sub(cs.ns(|| "sub v1"), &v1)?
        };

        let c2 = {
            a0_plus_a2
                .mul(cs.ns(|| "(a0 + a2) * (b0 + b2)"), &b0_plus_b2)?
                .sub(cs.ns(|| "sub v0"), &v0)?
                .add(cs.ns(|| "add v1"), &v1)?
        };

        Ok(Self::new(c0, c1, c2))
    }
}

type ConstraintPair<T> = (ConstraintVar<T>, ConstraintVar<T>);

impl<P, F: PrimeField> FieldGadget<Fp6<P>, F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    type Variable = (ConstraintPair<F>, ConstraintPair<F>, ConstraintPair<F>);

    #[inline]
    fn get_value(&self) -> Option<Fp6<P>> {
        match (self.c0.get_value(), self.c1.get_value(), self.c2.get_value()) {
            (Some(c0), Some(c1), Some(c2)) => Some(Fp6::new(c0, c1, c2)),
            (..) => None,
        }
    }

    #[inline]
    fn get_variable(&self) -> Self::Variable {
        (self.c0.get_variable(), self.c1.get_variable(), self.c2.get_variable())
    }

    #[inline]
    fn zero<CS: ConstraintSystem<F>>(mut cs: CS) -> Result<Self, SynthesisError> {
        let c0 = Fp2Gadget::<P, F>::zero(cs.ns(|| "c0"))?;
        let c1 = Fp2Gadget::<P, F>::zero(cs.ns(|| "c1"))?;
        let c2 = Fp2Gadget::<P, F>::zero(cs.ns(|| "c2"))?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn one<CS: ConstraintSystem<F>>(mut cs: CS) -> Result<Self, SynthesisError> {
        let c0 = Fp2Gadget::<P, F>::one(cs.ns(|| "c0"))?;
        let c1 = Fp2Gadget::<P, F>::zero(cs.ns(|| "c1"))?;
        let c2 = Fp2Gadget::<P, F>::zero(cs.ns(|| "c2"))?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn conditionally_add_constant<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        bit: &Boolean,
        coeff: Fp6<P>,
    ) -> Result<Self, SynthesisError> {
        let c0 = self.c0.conditionally_add_constant(cs.ns(|| "c0"), bit, coeff.c0)?;
        let c1 = self.c1.conditionally_add_constant(cs.ns(|| "c1"), bit, coeff.c1)?;
        let c2 = self.c2.conditionally_add_constant(cs.ns(|| "c2"), bit, coeff.c2)?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn add<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let c0 = self.c0.add(&mut cs.ns(|| "add c0"), &other.c0)?;
        let c1 = self.c1.add(&mut cs.ns(|| "add c1"), &other.c1)?;
        let c2 = self.c2.add(&mut cs.ns(|| "add c2"), &other.c2)?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn sub<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let c0 = self.c0.sub(&mut cs.ns(|| "sub c0"), &other.c0)?;
        let c1 = self.c1.sub(&mut cs.ns(|| "sub c1"), &other.c1)?;
        let c2 = self.c2.sub(&mut cs.ns(|| "sub c2"), &other.c2)?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn negate<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        let c0 = self.c0.negate(&mut cs.ns(|| "negate c0"))?;
        let c1 = self.c1.negate(&mut cs.ns(|| "negate c1"))?;
        let c2 = self.c2.negate(&mut cs.ns(|| "negate c2"))?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn negate_in_place<CS: ConstraintSystem<F>>(&mut self, mut cs: CS) -> Result<&mut Self, SynthesisError> {
        self.c0.negate_in_place(&mut cs.ns(|| "negate c0"))?;
        self.c1.negate_in_place(&mut cs.ns(|| "negate c1"))?;
        self.c2.negate_in_place(&mut cs.ns(|| "negate c2"))?;
        Ok(self)
    }

    /// Use the Toom-Cook-3x method to compute multiplication.
    #[inline]
    fn mul<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        // Uses Toom-Cool-3x multiplication from
        //
        // Reference:
        // "Multiplication and Squaring on Pairing-Friendly Fields"
        //    Devegili, OhEigeartaigh, Scott, Dahab

        // v0 = a(0)b(0)   = a0 * b0
        let v0 = self.c0.mul(&mut cs.ns(|| "Calc v0"), &other.c0)?;

        // v1 = a(1)b(1)   = (a0 + a1 + a2)(b0 + b1 + b2)
        let v1 = {
            let mut v1_cs = cs.ns(|| "compute v1");
            let a0_plus_a1_plus_a2 = self
                .c0
                .add(v1_cs.ns(|| "a0 + a1"), &self.c1)?
                .add(v1_cs.ns(|| "a0 + a1 + a2"), &self.c2)?;
            let b0_plus_b1_plus_b2 = other
                .c0
                .add(v1_cs.ns(|| "b0 + b1"), &other.c1)?
                .add(v1_cs.ns(|| "b0 + b1 + b2"), &other.c2)?;

            a0_plus_a1_plus_a2.mul(v1_cs.ns(|| "(a0 + a1 + a2)(b0 + b1 + b2)"), &b0_plus_b1_plus_b2)?
        };

        // v2 = a(−1)b(−1) = (a0 − a1 + a2)(b0 − b1 + b2)
        let v2 = {
            let mut v2_cs = cs.ns(|| "compute v2");

            let a0_minus_a1_plus_a2 = self
                .c0
                .sub(v2_cs.ns(|| "a0 - a1"), &self.c1)?
                .add(v2_cs.ns(|| "a0 - a1 + a2"), &self.c2)?;

            let b0_minus_b1_plus_b2 = other
                .c0
                .sub(v2_cs.ns(|| "b0 - b1"), &other.c1)?
                .add(v2_cs.ns(|| "b0 - b1 + b2"), &other.c2)?;

            a0_minus_a1_plus_a2.mul(v2_cs.ns(|| "(a0 - a1 + a2)(b0 - b1 + b2)"), &b0_minus_b1_plus_b2)?
        };

        // v3 = a(2)b(2)   = (a0 + 2a1 + 4a2)(b0 + 2b1 + 4b2)
        let v3 = {
            let v3_cs = &mut cs.ns(|| "compute v3");

            let a1_double = self.c1.double(v3_cs.ns(|| "2 * a1"))?;
            let a2_quad = self.c2.double(v3_cs.ns(|| "2 * a2"))?.double(v3_cs.ns(|| "4 * a2"))?;

            let a0_plus_2_a1_plus_4_a2 = self
                .c0
                .add(v3_cs.ns(|| "a0 + 2a1"), &a1_double)?
                .add(v3_cs.ns(|| "a0 + 2a1 + 4a2"), &a2_quad)?;

            let b1_double = other.c1.double(v3_cs.ns(|| "2 * b1"))?;
            let b2_quad = other.c2.double(v3_cs.ns(|| "2 * b2"))?.double(v3_cs.ns(|| "4 * b2"))?;
            let b0_plus_2_b1_plus_4_b2 = other
                .c0
                .add(v3_cs.ns(|| "b0 + 2b1"), &b1_double)?
                .add(v3_cs.ns(|| "b0 + 2b1 + 4b2"), &b2_quad)?;

            a0_plus_2_a1_plus_4_a2.mul(v3_cs.ns(|| "(a0 + 2a1 + 4a2)(b0 + 2b1 + 4b2)"), &b0_plus_2_b1_plus_4_b2)?
        };

        // v4 = a(∞)b(∞)   = a2 * b2
        let v4 = self.c2.mul(cs.ns(|| "v2: a2 * b2"), &other.c2)?;

        let two = <P::Fp2Params as Fp2Parameters>::Fp::one().double();
        let six = two.double() + &two;
        let mut two_and_six = [two, six];
        batch_inversion(&mut two_and_six);
        let (two_inverse, six_inverse) = (two_and_six[0], two_and_six[1]);

        let half_v0 = v0.mul_by_fp_constant(cs.ns(|| "half_v0"), &two_inverse)?;
        let half_v1 = v1.mul_by_fp_constant(cs.ns(|| "half_v1"), &two_inverse)?;
        let one_sixth_v2 = v2.mul_by_fp_constant(cs.ns(|| "v2_by_six"), &six_inverse)?;
        let one_sixth_v3 = v3.mul_by_fp_constant(cs.ns(|| "v3_by_six"), &six_inverse)?;
        let two_v4 = v4.double(cs.ns(|| "2 * v4"))?;

        // c0 = v0 + β((1/2)v0 − (1/2)v1 − (1/6)v2 + (1/6)v3 − 2v4)
        let c0 = {
            let c0_cs = &mut cs.ns(|| "c0");

            // No constraints, only get a linear combination back.
            let temp = half_v0
                .sub(c0_cs.ns(|| "sub1"), &half_v1)?
                .sub(c0_cs.ns(|| "sub2"), &one_sixth_v2)?
                .add(c0_cs.ns(|| "add3"), &one_sixth_v3)?
                .sub(c0_cs.ns(|| "sub4"), &two_v4)?;
            let non_residue_times_inner = temp.mul_by_constant(&mut c0_cs.ns(|| "mul5"), &P::NONRESIDUE)?;
            v0.add(c0_cs.ns(|| "add6"), &non_residue_times_inner)?
        };

        // −(1/2)v0 + v1 − (1/3)v2 − (1/6)v3 + 2v4 + βv4
        let c1 = {
            let c1_cs = &mut cs.ns(|| "c1");
            let one_third_v2 = one_sixth_v2.double(&mut c1_cs.ns(|| "v2_by_3"))?;
            let non_residue_v4 = v4.mul_by_constant(&mut c1_cs.ns(|| "mul_by_beta"), &P::NONRESIDUE)?;

            let result = half_v0
                .negate(c1_cs.ns(|| "neg1"))?
                .add(c1_cs.ns(|| "add2"), &v1)?
                .sub(c1_cs.ns(|| "sub3"), &one_third_v2)?
                .sub(c1_cs.ns(|| "sub4"), &one_sixth_v3)?
                .add(c1_cs.ns(|| "sub5"), &two_v4)?
                .add(c1_cs.ns(|| "sub6"), &non_residue_v4)?;
            result
        };

        // -v0 + (1/2)v1 + (1/2)v2 −v4
        let c2 = {
            let c2_cs = &mut cs.ns(|| "c2");
            let half_v2 = v2.mul_by_fp_constant(&mut c2_cs.ns(|| "mul1"), &two_inverse)?;
            let result = half_v1
                .add(c2_cs.ns(|| "add1"), &half_v2)?
                .sub(c2_cs.ns(|| "sub1"), &v4)?
                .sub(c2_cs.ns(|| "sub2"), &v0)?;
            result
        };

        Ok(Self::new(c0, c1, c2))
    }

    /// Use the Toom-Cook-3x method to compute multiplication.
    #[inline]
    fn square<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        // Uses Toom-Cool-3x multiplication from
        //
        // Reference:
        // "Multiplication and Squaring on Pairing-Friendly Fields"
        //    Devegili, OhEigeartaigh, Scott, Dahab

        // v0 = a(0)^2 = a0^2
        let v0 = self.c0.square(&mut cs.ns(|| "Calc v0"))?;

        // v1 = a(1)^2 = (a0 + a1 + a2)^2
        let v1 = {
            let a0_plus_a1_plus_a2 = self
                .c0
                .add(cs.ns(|| "a0 + a1"), &self.c1)?
                .add(cs.ns(|| "a0 + a1 + a2"), &self.c2)?;
            a0_plus_a1_plus_a2.square(&mut cs.ns(|| "(a0 + a1 + a2)^2"))?
        };

        // v2 = a(−1)^2 = (a0 − a1 + a2)^2
        let v2 = {
            let a0_minus_a1_plus_a2 = self
                .c0
                .sub(cs.ns(|| "a0 - a1"), &self.c1)?
                .add(cs.ns(|| "a0 - a2 + a2"), &self.c2)?;
            a0_minus_a1_plus_a2.square(&mut cs.ns(|| "(a0 - a1 + a2)^2"))?
        };

        // v3 = a(2)^2 = (a0 + 2a1 + 4a2)^2
        let v3 = {
            let a1_double = self.c1.double(cs.ns(|| "2a1"))?;
            let a2_quad = self.c2.double(cs.ns(|| "2a2"))?.double(cs.ns(|| "4a2"))?;
            let a0_plus_2_a1_plus_4_a2 = self
                .c0
                .add(cs.ns(|| "a0 + 2a1"), &a1_double)?
                .add(cs.ns(|| "a0 + 2a1 + 4a2"), &a2_quad)?;

            a0_plus_2_a1_plus_4_a2.square(&mut cs.ns(|| "(a0 + 2a1 + 4a2)^2"))?
        };

        // v4 = a(∞)^2 = a2^2
        let v4 = self.c2.square(&mut cs.ns(|| "a2^2"))?;

        let two = <P::Fp2Params as Fp2Parameters>::Fp::one().double();
        let six = two.double() + &two;
        let mut two_and_six = [two, six];
        batch_inversion(&mut two_and_six);
        let (two_inverse, six_inverse) = (two_and_six[0], two_and_six[1]);

        let half_v0 = v0.mul_by_fp_constant(cs.ns(|| "half_v0"), &two_inverse)?;
        let half_v1 = v1.mul_by_fp_constant(cs.ns(|| "half_v1"), &two_inverse)?;
        let one_sixth_v2 = v2.mul_by_fp_constant(cs.ns(|| "one_sixth_v2"), &six_inverse)?;
        let one_sixth_v3 = v3.mul_by_fp_constant(cs.ns(|| "one_sixth_v3"), &six_inverse)?;
        let two_v4 = v4.double(cs.ns(|| "double_v4"))?;

        // c0 = v0 + β((1/2)v0 − (1/2)v1 − (1/6)v2 + (1/6)v3 − 2v4)
        let c0 = {
            let mut c0_cs = cs.ns(|| "c0");
            // No constraints, only get a linear combination back.
            let inner = half_v0
                .sub(c0_cs.ns(|| "sub1"), &half_v1)?
                .sub(c0_cs.ns(|| "sub2"), &one_sixth_v2)?
                .add(c0_cs.ns(|| "add3"), &one_sixth_v3)?
                .sub(c0_cs.ns(|| "sub4"), &two_v4)?;
            let non_residue_times_inner = inner.mul_by_constant(c0_cs.ns(|| "mul_by_res"), &P::NONRESIDUE)?;
            v0.add(c0_cs.ns(|| "add5"), &non_residue_times_inner)?
        };

        // −(1/2)v0 + v1 − (1/3)v2 − (1/6)v3 + 2v4 + βv4
        let c1 = {
            let mut c1_cs = cs.ns(|| "c1");
            let one_third_v2 = one_sixth_v2.double(c1_cs.ns(|| "v2_by_3"))?;
            let non_residue_v4 = v4.mul_by_constant(c1_cs.ns(|| "mul_by_res"), &P::NONRESIDUE)?;

            half_v0
                .negate(c1_cs.ns(|| "neg1"))?
                .add(c1_cs.ns(|| "add1"), &v1)?
                .sub(c1_cs.ns(|| "sub2"), &one_third_v2)?
                .sub(c1_cs.ns(|| "sub3"), &one_sixth_v3)?
                .add(c1_cs.ns(|| "add4"), &two_v4)?
                .add(c1_cs.ns(|| "add5"), &non_residue_v4)?
        };

        // -v0 + (1/2)v1 + (1/2)v2 −v4
        let c2 = {
            let mut c2_cs = cs.ns(|| "c2");
            let half_v2 = v2.mul_by_fp_constant(c2_cs.ns(|| "half_v2"), &two_inverse)?;
            half_v1
                .add(c2_cs.ns(|| "add1"), &half_v2)?
                .sub(c2_cs.ns(|| "sub1"), &v4)?
                .sub(c2_cs.ns(|| "sub2"), &v0)?
        };

        Ok(Self::new(c0, c1, c2))
    }

    // 18 constaints, we can probably do better but not sure it's worth it.
    #[inline]
    fn inverse<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        let inverse = Self::alloc(&mut cs.ns(|| "alloc inverse"), || {
            self.get_value().and_then(|val| val.inverse()).get()
        })?;
        let one = Self::one(cs.ns(|| "one"))?;
        inverse.mul_equals(cs.ns(|| "check inverse"), &self, &one)?;
        Ok(inverse)
    }

    #[inline]
    fn add_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Fp6<P>) -> Result<Self, SynthesisError> {
        let c0 = self.c0.add_constant(cs.ns(|| "c0"), &other.c0)?;
        let c1 = self.c1.add_constant(cs.ns(|| "c1"), &other.c1)?;
        let c2 = self.c2.add_constant(cs.ns(|| "c2"), &other.c2)?;

        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn add_constant_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        mut cs: CS,
        other: &Fp6<P>,
    ) -> Result<&mut Self, SynthesisError> {
        self.c0.add_constant_in_place(cs.ns(|| "c0"), &other.c0)?;
        self.c1.add_constant_in_place(cs.ns(|| "c1"), &other.c1)?;
        self.c2.add_constant_in_place(cs.ns(|| "c2"), &other.c2)?;
        Ok(self)
    }

    /// Use the Toom-Cook-3x method to compute multiplication.
    #[inline]
    fn mul_by_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Fp6<P>) -> Result<Self, SynthesisError> {
        // Uses Toom-Cook-3x multiplication from
        //
        // Reference:
        // "Multiplication and Squaring on Pairing-Friendly Fields"
        //    Devegili, OhEigeartaigh, Scott, Dahab

        // v0 = a(0)b(0)   = a0 * b0
        let v0 = self.c0.mul_by_constant(cs.ns(|| "v0"), &other.c0)?;

        // v1 = a(1)b(1)   = (a0 + a1 + a2)(b0 + b1 + b2)
        let v1 = {
            let mut v1_cs = cs.ns(|| "v1");
            let mut a0_plus_a1_plus_a2 = self
                .c0
                .add(v1_cs.ns(|| "a0 + a1"), &self.c1)?
                .add(v1_cs.ns(|| "a0 + a1 + a2"), &self.c2)?;
            let b0_plus_b1_plus_b2 = other.c0 + &other.c1 + &other.c2;

            a0_plus_a1_plus_a2
                .mul_by_constant_in_place(v1_cs.ns(|| "(a0 + a1 + a2)*(b0 + b1 + b2)"), &b0_plus_b1_plus_b2)?;
            a0_plus_a1_plus_a2
        };

        // v2 = a(−1)b(−1) = (a0 − a1 + a2)(b0 − b1 + b2)
        let mut v2 = {
            let mut v2_cs = cs.ns(|| "v2");
            let mut a0_minus_a1_plus_a2 = self
                .c0
                .sub(v2_cs.ns(|| "sub1"), &self.c1)?
                .add(v2_cs.ns(|| "add2"), &self.c2)?;
            let b0_minus_b1_plus_b2 = other.c0 - &other.c1 + &other.c2;
            a0_minus_a1_plus_a2
                .mul_by_constant_in_place(v2_cs.ns(|| "(a0 - a1 + a2)*(b0 - b1 + b2)"), &b0_minus_b1_plus_b2)?;
            a0_minus_a1_plus_a2
        };

        // v3 = a(2)b(2)   = (a0 + 2a1 + 4a2)(b0 + 2b1 + 4b2)
        let mut v3 = {
            let mut v3_cs = cs.ns(|| "v3");
            let a1_double = self.c1.double(v3_cs.ns(|| "2a1"))?;
            let a2_quad = self.c2.double(v3_cs.ns(|| "2a2"))?.double(v3_cs.ns(|| "4a2"))?;
            let mut a0_plus_2_a1_plus_4_a2 = self
                .c0
                .add(v3_cs.ns(|| "a0 + 2a1"), &a1_double)?
                .add(v3_cs.ns(|| "a0 + 2a1 + 4a2"), &a2_quad)?;

            let b1_double = other.c1.double();
            let b2_quad = other.c2.double().double();
            let b0_plus_2_b1_plus_4_b2 = other.c0 + &b1_double + &b2_quad;

            a0_plus_2_a1_plus_4_a2.mul_by_constant_in_place(
                v3_cs.ns(|| "(a0 + 2a1 + 4a2)*(b0 + 2b1 + 4b2)"),
                &b0_plus_2_b1_plus_4_b2,
            )?;
            a0_plus_2_a1_plus_4_a2
        };

        // v4 = a(∞)b(∞)   = a2 * b2
        let v4 = self.c2.mul_by_constant(cs.ns(|| "v4"), &other.c2)?;

        let two = <P::Fp2Params as Fp2Parameters>::Fp::one().double();
        let six = two.double() + &two;
        let mut two_and_six = [two, six];
        batch_inversion(&mut two_and_six);
        let (two_inverse, six_inverse) = (two_and_six[0], two_and_six[1]);

        let mut half_v0 = v0.mul_by_fp_constant(cs.ns(|| "half_v0"), &two_inverse)?;
        let half_v1 = v1.mul_by_fp_constant(cs.ns(|| "half_v1"), &two_inverse)?;
        let mut one_sixth_v2 = v2.mul_by_fp_constant(cs.ns(|| "v2_by_6"), &six_inverse)?;
        let one_sixth_v3 = v3.mul_by_fp_constant_in_place(cs.ns(|| "v3_by_6"), &six_inverse)?;
        let two_v4 = v4.double(cs.ns(|| "2v4"))?;

        // c0 = v0 + β((1/2)v0 − (1/2)v1 − (1/6)v2 + (1/6)v3 − 2v4)
        let c0 = {
            let mut c0_cs = cs.ns(|| "c0");

            // No constraints, only get a linear combination back.
            let mut inner = half_v0
                .sub(c0_cs.ns(|| "sub1"), &half_v1)?
                .sub(c0_cs.ns(|| "sub2"), &one_sixth_v2)?
                .add(c0_cs.ns(|| "add3"), &one_sixth_v3)?
                .sub(c0_cs.ns(|| "sub4"), &two_v4)?;
            let non_residue_times_inner = inner.mul_by_constant_in_place(&mut c0_cs, &P::NONRESIDUE)?;
            v0.add(c0_cs.ns(|| "add5"), non_residue_times_inner)?
        };

        // −(1/2)v0 + v1 − (1/3)v2 − (1/6)v3 + 2v4 + βv4
        let c1 = {
            let mut c1_cs = cs.ns(|| "c1");
            let one_third_v2 = one_sixth_v2.double_in_place(c1_cs.ns(|| "double1"))?;
            let non_residue_v4 = v4.mul_by_constant(c1_cs.ns(|| "mul_by_const1"), &P::NONRESIDUE)?;

            half_v0
                .negate_in_place(c1_cs.ns(|| "neg1"))?
                .add(c1_cs.ns(|| "add1"), &v1)?
                .sub(c1_cs.ns(|| "sub2"), one_third_v2)?
                .sub(c1_cs.ns(|| "sub3"), &one_sixth_v3)?
                .add(c1_cs.ns(|| "add4"), &two_v4)?
                .add(c1_cs.ns(|| "add5"), &non_residue_v4)?
        };

        // -v0 + (1/2)v1 + (1/2)v2 −v4
        let c2 = {
            let mut c2_cs = cs.ns(|| "c2");
            let half_v2 = v2.mul_by_fp_constant_in_place(c2_cs.ns(|| "half_v2"), &two_inverse)?;
            half_v1
                .add(c2_cs.ns(|| "add1"), half_v2)?
                .sub(c2_cs.ns(|| "sub2"), &v4)?
                .sub(c2_cs.ns(|| "sub3"), &v0)?
        };

        Ok(Self::new(c0, c1, c2))
    }

    fn frobenius_map<CS: ConstraintSystem<F>>(&self, cs: CS, power: usize) -> Result<Self, SynthesisError> {
        let mut result = self.clone();
        result.frobenius_map_in_place(cs, power)?;
        Ok(result)
    }

    fn frobenius_map_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        mut cs: CS,
        power: usize,
    ) -> Result<&mut Self, SynthesisError> {
        self.c0.frobenius_map_in_place(&mut cs.ns(|| "c0"), power)?;
        self.c1.frobenius_map_in_place(&mut cs.ns(|| "c1"), power)?;
        self.c2.frobenius_map_in_place(&mut cs.ns(|| "c2"), power)?;

        self.c1
            .mul_by_constant_in_place(cs.ns(|| "c1_power"), &P::FROBENIUS_COEFF_FP6_C1[power % 6])?;
        self.c2
            .mul_by_constant_in_place(cs.ns(|| "c2_power"), &P::FROBENIUS_COEFF_FP6_C2[power % 6])?;

        Ok(self)
    }

    fn cost_of_mul() -> usize {
        5 * Fp2Gadget::<P, F>::cost_of_mul()
    }

    fn cost_of_inv() -> usize {
        Self::cost_of_mul() + <Self as EqGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> PartialEq for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.c0 == other.c0 && self.c1 == other.c1 && self.c2 == other.c2
    }
}

impl<P, F: PrimeField> Eq for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
}

impl<P, F: PrimeField> EqGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
}

impl<P, F: PrimeField> ConditionalEqGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        self.c0
            .conditional_enforce_equal(&mut cs.ns(|| "c0"), &other.c0, condition)?;
        self.c1
            .conditional_enforce_equal(&mut cs.ns(|| "c1"), &other.c1, condition)?;
        self.c2
            .conditional_enforce_equal(&mut cs.ns(|| "c2"), &other.c2, condition)?;
        Ok(())
    }

    fn cost() -> usize {
        3 * <Fp2Gadget<P, F> as ConditionalEqGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> NEqGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn enforce_not_equal<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<(), SynthesisError> {
        self.c0.enforce_not_equal(&mut cs.ns(|| "c0"), &other.c0)?;
        self.c1.enforce_not_equal(&mut cs.ns(|| "c1"), &other.c1)?;
        self.c2.enforce_not_equal(&mut cs.ns(|| "c2"), &other.c2)?;
        Ok(())
    }

    fn cost() -> usize {
        3 * <Fp2Gadget<P, F> as NEqGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> ToBitsGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn to_bits<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut c0 = self.c0.to_bits(&mut cs)?;
        let mut c1 = self.c1.to_bits(&mut cs)?;
        let mut c2 = self.c2.to_bits(cs)?;

        c0.append(&mut c1);
        c0.append(&mut c2);

        Ok(c0)
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut c0 = self.c0.to_bits_strict(&mut cs)?;
        let mut c1 = self.c1.to_bits_strict(&mut cs)?;
        let mut c2 = self.c2.to_bits_strict(cs)?;

        c0.append(&mut c1);
        c0.append(&mut c2);

        Ok(c0)
    }
}

impl<P, F: PrimeField> ToBytesGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut c0 = self.c0.to_bytes(cs.ns(|| "c0"))?;
        let mut c1 = self.c1.to_bytes(cs.ns(|| "c1"))?;
        let mut c2 = self.c2.to_bytes(cs.ns(|| "c2"))?;

        c0.append(&mut c1);
        c0.append(&mut c2);

        Ok(c0)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        self.to_bytes(cs)
    }
}

impl<P, F: PrimeField> Clone for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn clone(&self) -> Self {
        Self::new(self.c0.clone(), self.c1.clone(), self.c2.clone())
    }
}

impl<P, F: PrimeField> CondSelectGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn conditionally_select<CS: ConstraintSystem<F>>(
        mut cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError> {
        let c0 = Fp2Gadget::<P, F>::conditionally_select(&mut cs.ns(|| "c0"), cond, &first.c0, &second.c0)?;
        let c1 = Fp2Gadget::<P, F>::conditionally_select(&mut cs.ns(|| "c1"), cond, &first.c1, &second.c1)?;
        let c2 = Fp2Gadget::<P, F>::conditionally_select(&mut cs.ns(|| "c2"), cond, &first.c2, &second.c2)?;

        Ok(Self::new(c0, c1, c2))
    }

    fn cost() -> usize {
        3 * <Fp2Gadget<P, F> as CondSelectGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> TwoBitLookupGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    type TableConstant = Fp6<P>;

    fn two_bit_lookup<CS: ConstraintSystem<F>>(
        mut cs: CS,
        b: &[Boolean],
        c: &[Self::TableConstant],
    ) -> Result<Self, SynthesisError> {
        let c0s = c.iter().map(|f| f.c0).collect::<Vec<_>>();
        let c1s = c.iter().map(|f| f.c1).collect::<Vec<_>>();
        let c2s = c.iter().map(|f| f.c2).collect::<Vec<_>>();
        let c0 = Fp2Gadget::<P, F>::two_bit_lookup(cs.ns(|| "Lookup c0"), b, &c0s)?;
        let c1 = Fp2Gadget::<P, F>::two_bit_lookup(cs.ns(|| "Lookup c1"), b, &c1s)?;
        let c2 = Fp2Gadget::<P, F>::two_bit_lookup(cs.ns(|| "Lookup c2"), b, &c2s)?;
        Ok(Self::new(c0, c1, c2))
    }

    fn cost() -> usize {
        3 * <Fp2Gadget<P, F> as TwoBitLookupGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> ThreeBitCondNegLookupGadget<F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    type TableConstant = Fp6<P>;

    fn three_bit_cond_neg_lookup<CS: ConstraintSystem<F>>(
        mut cs: CS,
        b: &[Boolean],
        b0b1: &Boolean,
        c: &[Self::TableConstant],
    ) -> Result<Self, SynthesisError> {
        let c0s = c.iter().map(|f| f.c0).collect::<Vec<_>>();
        let c1s = c.iter().map(|f| f.c1).collect::<Vec<_>>();
        let c2s = c.iter().map(|f| f.c2).collect::<Vec<_>>();
        let c0 = Fp2Gadget::<P, F>::three_bit_cond_neg_lookup(cs.ns(|| "Lookup c0"), b, b0b1, &c0s)?;
        let c1 = Fp2Gadget::<P, F>::three_bit_cond_neg_lookup(cs.ns(|| "Lookup c1"), b, b0b1, &c1s)?;
        let c2 = Fp2Gadget::<P, F>::three_bit_cond_neg_lookup(cs.ns(|| "Lookup c2"), b, b0b1, &c2s)?;
        Ok(Self::new(c0, c1, c2))
    }

    fn cost() -> usize {
        3 * <Fp2Gadget<P, F> as ThreeBitCondNegLookupGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> AllocGadget<Fp6<P>, F> for Fp6Gadget<P, F>
where
    P: Fp6Parameters,
    P::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<Fp6<P>>,
    {
        let (c0, c1, c2) = match value_gen() {
            Ok(fe) => {
                let fe = *fe.borrow();
                (Ok(fe.c0), Ok(fe.c1), Ok(fe.c2))
            }
            _ => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        let c0 = Fp2Gadget::<P, F>::alloc(&mut cs.ns(|| "c0"), || c0)?;
        let c1 = Fp2Gadget::<P, F>::alloc(&mut cs.ns(|| "c1"), || c1)?;
        let c2 = Fp2Gadget::<P, F>::alloc(&mut cs.ns(|| "c2"), || c2)?;
        Ok(Self::new(c0, c1, c2))
    }

    #[inline]
    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<Fp6<P>>,
    {
        let (c0, c1, c2) = match value_gen() {
            Ok(fe) => {
                let fe = *fe.borrow();
                (Ok(fe.c0), Ok(fe.c1), Ok(fe.c2))
            }
            _ => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        let c0 = Fp2Gadget::<P, F>::alloc_input(&mut cs.ns(|| "c0"), || c0)?;
        let c1 = Fp2Gadget::<P, F>::alloc_input(&mut cs.ns(|| "c1"), || c1)?;
        let c2 = Fp2Gadget::<P, F>::alloc_input(&mut cs.ns(|| "c2"), || c2)?;
        Ok(Self::new(c0, c1, c2))
    }
}
