use crate::{
    curves::{
        fp12_2over3over2::{Fp12, Fp12Parameters},
        fp6_3over2::{Fp6, Fp6Parameters},
        Field, Fp2Parameters, PrimeField,
    },
    gadgets::{
        curves::FieldGadget,
        r1cs::{Assignment, ConstraintSystem, SynthesisError},
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::{ConditionalEqGadget, EqGadget, NEqGadget},
            select::{CondSelectGadget, ThreeBitCondNegLookupGadget, TwoBitLookupGadget},
            uint8::UInt8,
            ToBitsGadget, ToBytesGadget,
        },
    },
};
use snarkos_utilities::bititerator::BitIterator;

use std::{borrow::Borrow, marker::PhantomData};

type Fp2Gadget<P, F> = super::fp2::Fp2Gadget<<<P as Fp12Parameters>::Fp6Params as Fp6Parameters>::Fp2Params, F>;
type Fp6Gadget<P, F> = super::fp6_3over2::Fp6Gadget<<P as Fp12Parameters>::Fp6Params, F>;
type Fp6GadgetVariable<P, F> = <Fp6Gadget<P, F> as FieldGadget<Fp6<<P as Fp12Parameters>::Fp6Params>, F>>::Variable;

#[derive(Derivative)]
#[derivative(Debug(bound = "F: PrimeField"))]
#[must_use]
pub struct Fp12Gadget<P, F: PrimeField>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    pub c0: Fp6Gadget<P, F>,
    pub c1: Fp6Gadget<P, F>,
    #[derivative(Debug = "ignore")]
    _params: PhantomData<P>,
}

impl<P, F: PrimeField> Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    pub fn new(c0: Fp6Gadget<P, F>, c1: Fp6Gadget<P, F>) -> Self {
        Self {
            c0,
            c1,
            _params: PhantomData,
        }
    }

    /// Multiply by quadratic nonresidue v.
    #[inline]
    pub(crate) fn mul_fp6_by_nonresidue<CS: ConstraintSystem<F>>(
        cs: CS,
        fe: &Fp6Gadget<P, F>,
    ) -> Result<Fp6Gadget<P, F>, SynthesisError> {
        let new_c0 = Fp6Gadget::<P, F>::mul_fp2_gadget_by_nonresidue(cs, &fe.c2)?;
        let new_c1 = fe.c0.clone();
        let new_c2 = fe.c1.clone();
        Ok(Fp6Gadget::<P, F>::new(new_c0, new_c1, new_c2))
    }

    #[inline]
    pub fn conjugate_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS) -> Result<&mut Self, SynthesisError> {
        self.c1.negate_in_place(cs)?;
        Ok(self)
    }

    /// Multiplies by an element of the form (c0 = (c0, c1, 0), c1 = (0, d1, 0))
    #[inline]
    pub fn mul_by_014<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        c0: &Fp2Gadget<P, F>,
        c1: &Fp2Gadget<P, F>,
        d1: &Fp2Gadget<P, F>,
    ) -> Result<Self, SynthesisError> {
        let v0 = self.c0.mul_by_c0_c1_0(cs.ns(|| "v0"), &c0, &c1)?;
        let v1 = self.c1.mul_by_0_c1_0(cs.ns(|| "v1"), &d1)?;
        let new_c0 = Self::mul_fp6_by_nonresidue(cs.ns(|| "first mul_by_nr"), &v1)?
            .add(cs.ns(|| "v0 + nonresidue * v1"), &v0)?;

        let c1 = {
            let tmp = c1.add(cs.ns(|| "c1 + d1"), &d1)?;
            let a0_plus_a1 = self.c0.add(cs.ns(|| "a0 + a1"), &self.c1)?;
            a0_plus_a1
                .mul_by_c0_c1_0(cs.ns(|| "(a0 + a1) * (b0 + b1)"), &c0, &tmp)?
                .sub(cs.ns(|| "sub v0"), &v0)?
                .sub(cs.ns(|| "sub v1"), &v1)?
        };
        Ok(Self::new(new_c0, c1))
    }

    /// Multiplies by an element of the form (c0 = (c0, 0, 0), c1 = (d0, d1, 0))
    #[inline]
    pub fn mul_by_034<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        c0: &Fp2Gadget<P, F>,
        d0: &Fp2Gadget<P, F>,
        d1: &Fp2Gadget<P, F>,
    ) -> Result<Self, SynthesisError> {
        let a0 = self.c0.c0.mul(cs.ns(|| "a0"), &c0)?;
        let a1 = self.c0.c1.mul(cs.ns(|| "a1"), &c0)?;
        let a2 = self.c0.c2.mul(cs.ns(|| "a2"), &c0)?;
        let a = Fp6Gadget::<P, F>::new(a0, a1, a2);
        let b = self.c1.mul_by_c0_c1_0(cs.ns(|| "b"), &d0, &d1)?;

        let c0 = c0.add(cs.ns(|| "c0 + d0"), &d0)?;
        let c1 = d1;
        let e =
            self.c0
                .add(cs.ns(|| "self.c0 + self.c1"), &self.c1)?
                .mul_by_c0_c1_0(cs.ns(|| "compute e"), &c0, &c1)?;
        let a_plus_b = a.add(cs.ns(|| "a + b"), &b)?;
        let c1 = e.sub(cs.ns(|| "e - (a + b)"), &a_plus_b)?;
        let c0 = Self::mul_fp6_by_nonresidue(cs.ns(|| "b *nr"), &b)?.add(cs.ns(|| "plus a"), &a)?;

        Ok(Self::new(c0, c1))
    }

    pub fn cyclotomic_square<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        let mut result = Self::zero(cs.ns(|| "alloc result"))?;
        let fp2_nr = <P::Fp6Params as Fp6Parameters>::NONRESIDUE;

        let z0 = &self.c0.c0;
        let z4 = &self.c0.c1;
        let z3 = &self.c0.c2;
        let z2 = &self.c1.c0;
        let z1 = &self.c1.c1;
        let z5 = &self.c1.c2;

        // t0 + t1*y = (z0 + z1*y)^2 = a^2
        let tmp = z0.mul(cs.ns(|| "first mul"), &z1)?;
        let t0 = {
            // (z0 + &z1) * &(z0 + &(fp2_nr * &z1)) - &tmp - &(tmp * &fp2_nr);
            let mut cs = cs.ns(|| "t0");
            let tmp1 = z0.add(cs.ns(|| "tmp1"), &z1)?;
            let tmp2 = z1
                .mul_by_constant(cs.ns(|| "tmp2.1"), &fp2_nr)?
                .add(cs.ns(|| "tmp2.2"), &z0)?;
            let tmp4 = tmp
                .mul_by_constant(cs.ns(|| "tmp4.1"), &fp2_nr)?
                .add(cs.ns(|| "tmp4.2"), &tmp)?;
            tmp1.mul(cs.ns(|| "tmp3.1"), &tmp2)?.sub(cs.ns(|| "tmp3.2"), &tmp4)?
        };
        let t1 = tmp.double(cs.ns(|| "t1"))?;

        // t2 + t3*y = (z2 + z3*y)^2 = b^2
        let tmp = z2.mul(cs.ns(|| "second mul"), &z3)?;
        let t2 = {
            // (z2 + &z3) * &(z2 + &(fp2_nr * &z3)) - &tmp - &(tmp * &fp2_nr);
            let mut cs = cs.ns(|| "t2");
            let tmp1 = z2.add(cs.ns(|| "tmp1"), &z3)?;
            let tmp2 = z3
                .mul_by_constant(cs.ns(|| "tmp2.1"), &fp2_nr)?
                .add(cs.ns(|| "tmp2.2"), &z2)?;
            let tmp4 = tmp
                .mul_by_constant(cs.ns(|| "tmp4.1"), &fp2_nr)?
                .add(cs.ns(|| "tmp4.2"), &tmp)?;
            tmp1.mul(cs.ns(|| "tmp3.1"), &tmp2)?.sub(cs.ns(|| "tmp3.2"), &tmp4)?
        };
        let t3 = tmp.double(cs.ns(|| "t3"))?;

        // t4 + t5*y = (z4 + z5*y)^2 = c^2
        let tmp = z4.mul(cs.ns(|| "third mul"), &z5)?;
        let t4 = {
            // (z4 + &z5) * &(z4 + &(fp2_nr * &z5)) - &tmp - &(tmp * &fp2_nr);
            let mut cs = cs.ns(|| "t4");
            let tmp1 = z4.add(cs.ns(|| "tmp1"), &z5)?;
            let tmp2 = z5
                .mul_by_constant(cs.ns(|| "tmp2.1"), &fp2_nr)?
                .add(cs.ns(|| "tmp2.2"), &z4)?;
            let tmp4 = tmp
                .mul_by_constant(cs.ns(|| "tmp4.1"), &fp2_nr)?
                .add(cs.ns(|| "tmp4.2"), &tmp)?;
            tmp1.mul(cs.ns(|| "tmp3.1"), &tmp2)?.sub(cs.ns(|| "tmp3.2"), &tmp4)?
        };
        let t5 = tmp.double(cs.ns(|| "t5"))?;

        // for A

        // z0 = 3 * t0 - 2 * z0
        result.c0.c0 = {
            let mut cs = cs.ns(|| "result.c0.c0");
            t0.sub(cs.ns(|| "1"), &z0)?
                .double(cs.ns(|| "2"))?
                .add(cs.ns(|| "3"), &t0)?
        };

        // z1 = 3 * t1 + 2 * z1
        result.c1.c1 = {
            let mut cs = cs.ns(|| "result.c1.c1");
            t1.add(cs.ns(|| "1"), &z1)?
                .double(cs.ns(|| "2"))?
                .add(cs.ns(|| "3"), &t1)?
        };

        // for B

        // z2 = 3 * (xi * t5) + 2 * z2
        result.c1.c0 = {
            let mut cs = cs.ns(|| "result.c1.c0");
            let tmp = t5.mul_by_constant(cs.ns(|| "1"), &fp2_nr)?;
            z2.add(cs.ns(|| "2"), &tmp)?
                .double(cs.ns(|| "3"))?
                .add(cs.ns(|| "4"), &tmp)?
        };

        // z3 = 3 * t4 - 2 * z3
        result.c0.c2 = {
            let mut cs = cs.ns(|| "result.c0.c2");
            t4.sub(cs.ns(|| "1"), &z3)?
                .double(cs.ns(|| "2"))?
                .add(cs.ns(|| "3"), &t4)?
        };

        // for C

        // z4 = 3 * t2 - 2 * z4
        result.c0.c1 = {
            let mut cs = cs.ns(|| "result.c0.c1");
            t2.sub(cs.ns(|| "1"), &z4)?
                .double(cs.ns(|| "2"))?
                .add(cs.ns(|| "3"), &t2)?
        };

        // z5 = 3 * t3 + 2 * z5
        result.c1.c2 = {
            let mut cs = cs.ns(|| "result.c1.c2");
            t3.add(cs.ns(|| "1"), &z5)?
                .double(cs.ns(|| "2"))?
                .add(cs.ns(|| "3"), &t3)?
        };

        Ok(result)
    }

    #[inline]
    pub fn cyclotomic_exp<CS: ConstraintSystem<F>, S: AsRef<[u64]>>(
        &self,
        mut cs: CS,
        exp: S,
    ) -> Result<Self, SynthesisError> {
        let mut res = Self::one(cs.ns(|| "one"))?;
        let mut found_one = false;
        for (j, i) in BitIterator::new(exp).enumerate() {
            if found_one {
                res = res.cyclotomic_square(cs.ns(|| format!("res_square_{:?}", j)))?;
            } else {
                found_one = i;
            }
            if i {
                res.mul_in_place(cs.ns(|| format!("res_mul2_{:?}", j)), self)?;
            }
        }
        Ok(res)
    }
}

impl<P, F: PrimeField> FieldGadget<Fp12<P>, F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    type Variable = (Fp6GadgetVariable<P, F>, Fp6GadgetVariable<P, F>);

    #[inline]
    fn get_value(&self) -> Option<Fp12<P>> {
        Some(Fp12::new(self.c0.get_value()?, self.c1.get_value()?))
    }

    #[inline]
    fn get_variable(&self) -> Self::Variable {
        (self.c0.get_variable(), self.c1.get_variable())
    }

    #[inline]
    fn zero<CS: ConstraintSystem<F>>(mut cs: CS) -> Result<Self, SynthesisError> {
        let c0 = Fp6Gadget::<P, F>::zero(cs.ns(|| "c0"))?;
        let c1 = Fp6Gadget::<P, F>::zero(cs.ns(|| "c1"))?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn one<CS: ConstraintSystem<F>>(mut cs: CS) -> Result<Self, SynthesisError> {
        let c0 = Fp6Gadget::<P, F>::one(cs.ns(|| "c0"))?;
        let c1 = Fp6Gadget::<P, F>::zero(cs.ns(|| "c1"))?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn conditionally_add_constant<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        bit: &Boolean,
        coeff: Fp12<P>,
    ) -> Result<Self, SynthesisError> {
        let c0 = self.c0.conditionally_add_constant(cs.ns(|| "c0"), bit, coeff.c0)?;
        let c1 = self.c1.conditionally_add_constant(cs.ns(|| "c1"), bit, coeff.c1)?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn add<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let c0 = self.c0.add(cs.ns(|| "c0"), &other.c0)?;
        let c1 = self.c1.add(cs.ns(|| "c1"), &other.c1)?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn add_in_place<CS: ConstraintSystem<F>>(&mut self, mut cs: CS, other: &Self) -> Result<&mut Self, SynthesisError> {
        self.c0.add_in_place(cs.ns(|| "c0"), &other.c0)?;
        self.c1.add_in_place(cs.ns(|| "c1"), &other.c1)?;
        Ok(self)
    }

    #[inline]
    fn sub<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let c0 = self.c0.sub(cs.ns(|| "c0"), &other.c0)?;
        let c1 = self.c1.sub(cs.ns(|| "c1"), &other.c1)?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn sub_in_place<CS: ConstraintSystem<F>>(&mut self, mut cs: CS, other: &Self) -> Result<&mut Self, SynthesisError> {
        self.c0.sub_in_place(cs.ns(|| "c0"), &other.c0)?;
        self.c1.sub_in_place(cs.ns(|| "c1"), &other.c1)?;
        Ok(self)
    }

    #[inline]
    fn negate<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        let c0 = self.c0.negate(cs.ns(|| "c0"))?;
        let c1 = self.c1.negate(cs.ns(|| "c1"))?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn negate_in_place<CS: ConstraintSystem<F>>(&mut self, mut cs: CS) -> Result<&mut Self, SynthesisError> {
        self.c0.negate_in_place(cs.ns(|| "c0"))?;
        self.c1.negate_in_place(cs.ns(|| "c1"))?;
        Ok(self)
    }

    #[inline]
    fn mul<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        // Karatsuba multiplication:
        // v0 = A.c0 * B.c0
        // v1 = A.c1 * B.c1
        // result.c0 = v0 + non_residue * v1
        // result.c1 = (A.c0 + A.c1) * (B.c0 + B.c1) - v0 - v1
        //
        // Enforced with 3 Fp3_mul_gadget's that ensure that:
        // A.c1 * B.c1 = v1
        // A.c0 * B.c0 = v0
        // (A.c0+A.c1)*(B.c0+B.c1) = result.c1 + v0 + v1

        let v0 = self.c0.mul(cs.ns(|| "v0"), &other.c0)?;
        let v1 = self.c1.mul(cs.ns(|| "v1"), &other.c1)?;
        let c0 = {
            let non_residue_times_v1 = Self::mul_fp6_by_nonresidue(cs.ns(|| "first mul_by_nr"), &v1)?;
            v0.add(cs.ns(|| "v0 + beta * v1"), &non_residue_times_v1)?
        };
        let c1 = {
            let a0_plus_a1 = self.c0.add(cs.ns(|| "a0 + a1"), &self.c1)?;
            let b0_plus_b1 = other.c0.add(cs.ns(|| "b0 + b1"), &other.c1)?;
            let a0_plus_a1_times_b0_plus_b1 = a0_plus_a1.mul(&mut cs.ns(|| "(a0 + a1) * (b0 + b1)"), &b0_plus_b1)?;
            a0_plus_a1_times_b0_plus_b1
                .sub(cs.ns(|| "res - v0"), &v0)?
                .sub(cs.ns(|| "res - v0 - v1"), &v1)?
        };

        Ok(Self::new(c0, c1))
    }

    fn square<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        // From Libsnark/fp2_gadget.tcc
        // Complex multiplication for Fp2:
        //     v0 = A.c0 * A.c1
        //     result.c0 = (A.c0 + A.c1) * (A.c0 + non_residue * A.c1) - (1 +
        // non_residue) * v0     result.c1 = 2 * v0
        // Enforced with 2 constraints:
        //     (2*A.c0) * A.c1 = result.c1
        //     (A.c0 + A.c1) * (A.c0 + non_residue * A.c1) = result.c0 + result.c1 * (1
        // + non_residue)/2 Reference:
        //     "Multiplication and Squaring on Pairing-Friendly Fields"
        //     Devegili, OhEigeartaigh, Scott, Dahab

        let mut v0 = self.c0.mul(cs.ns(|| "v0"), &self.c1)?;
        let a0_plus_a1 = self.c0.add(cs.ns(|| "a0 + a1"), &self.c1)?;

        let non_residue_a1 = Self::mul_fp6_by_nonresidue(cs.ns(|| "non_residue * a1"), &self.c1)?;
        let a0_plus_non_residue_a1 = self.c0.add(cs.ns(|| "a0 + non_residue * a1"), &non_residue_a1)?;
        let one_plus_non_residue_v0 =
            Self::mul_fp6_by_nonresidue(cs.ns(|| "non_residue * v0"), &v0)?.add(cs.ns(|| "plus v0"), &v0)?;

        let c0 = a0_plus_a1
            .mul(cs.ns(|| "(a0 + a1) * (a0 + non_residue * a1)"), &a0_plus_non_residue_a1)?
            .sub(cs.ns(|| "- (1 + non_residue) v0"), &one_plus_non_residue_v0)?;

        v0.double_in_place(cs.ns(|| "2v0"))?;
        let c1 = v0;

        Ok(Self {
            c0,
            c1,
            _params: PhantomData,
        })
    }

    #[inline]
    fn add_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Fp12<P>) -> Result<Self, SynthesisError> {
        let c0 = self.c0.add_constant(cs.ns(|| "c0"), &other.c0)?;
        let c1 = self.c1.add_constant(cs.ns(|| "c1"), &other.c1)?;

        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn add_constant_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        mut cs: CS,
        other: &Fp12<P>,
    ) -> Result<&mut Self, SynthesisError> {
        self.c0.add_constant_in_place(cs.ns(|| "c0"), &other.c0)?;
        self.c1.add_constant_in_place(cs.ns(|| "c1"), &other.c1)?;
        Ok(self)
    }

    fn mul_by_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Fp12<P>) -> Result<Self, SynthesisError> {
        // Karatsuba multiplication (see mul above).
        // Doesn't need any constraints; returns linear combinations of
        // `self`'s variables.
        //
        // (The operations below are guaranteed to return linear combinations)
        let (a0, a1) = (&self.c0, &self.c1);
        let (b0, b1) = (other.c0, other.c1);
        let mut v0 = a0.mul_by_constant(&mut cs.ns(|| "v0"), &b0)?;
        let mut v1 = Self::mul_fp6_by_nonresidue(&mut cs.ns(|| "v1"), a1)?;
        let beta_v1 = v1.mul_by_constant_in_place(&mut cs.ns(|| "beta * v1"), &b1)?;

        v0.add_in_place(&mut cs.ns(|| "c0"), beta_v1)?;
        let c0 = v0;

        let mut a0b1 = a0.mul_by_constant(&mut cs.ns(|| "a0b1"), &b1)?;
        let a1b0 = a1.mul_by_constant(&mut cs.ns(|| "a1b0"), &b0)?;
        a0b1.add_in_place(&mut cs.ns(|| "c1"), &a1b0)?;
        let c1 = a0b1;
        Ok(Self::new(c0, c1))
    }

    fn frobenius_map<CS: ConstraintSystem<F>>(&self, cs: CS, power: usize) -> Result<Self, SynthesisError> {
        let mut res = self.clone();
        res.frobenius_map_in_place(cs, power)?;
        Ok(res)
    }

    fn frobenius_map_in_place<CS: ConstraintSystem<F>>(
        &mut self,
        mut cs: CS,
        power: usize,
    ) -> Result<&mut Self, SynthesisError> {
        self.c0.frobenius_map_in_place(cs.ns(|| "frob_map1"), power)?;
        self.c1.frobenius_map_in_place(cs.ns(|| "frob_map2"), power)?;

        self.c1
            .c0
            .mul_by_constant_in_place(cs.ns(|| "mul1"), &P::FROBENIUS_COEFF_FP12_C1[power % 12])?;
        self.c1
            .c1
            .mul_by_constant_in_place(cs.ns(|| "mul2"), &P::FROBENIUS_COEFF_FP12_C1[power % 12])?;
        self.c1
            .c2
            .mul_by_constant_in_place(cs.ns(|| "mul3"), &P::FROBENIUS_COEFF_FP12_C1[power % 12])?;
        Ok(self)
    }

    fn inverse<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        let inverse = Self::alloc(&mut cs.ns(|| "alloc inverse"), || {
            self.get_value().and_then(|val| val.inverse()).get()
        })?;

        // Karatsuba multiplication for Fp2 with the inverse:
        //     v0 = A.c0 * B.c0
        //     v1 = A.c1 * B.c1
        //
        //      1 = v0 + non_residue * v1
        //  => v0 = 1 - non_residue * v1
        //
        //      0 = result.c1 = (A.c0 + A.c1) * (B.c0 + B.c1) - v0 - v1
        //  => v0 + v1 = (A.c0 + A.c1) * (B.c0 + B.c1)
        //  => 1 + (1 - non_residue) * v1 = (A.c0 + A.c1) * (B.c0 + B.c1)
        // Enforced with 2 constraints:
        //     A.c1 * B.c1 = v1
        //  => 1 + (1 - non_residue) * v1 = (A.c0 + A.c1) * (B.c0 + B.c1)
        // Reference:
        // "Multiplication and Squaring on Pairing-Friendly Fields"
        // Devegili, OhEigeartaigh, Scott, Dahab

        // Constraint 1
        let v1 = self.c1.mul(cs.ns(|| "inv_constraint_1"), &inverse.c1)?;

        // Constraint 2
        let a0_plus_a1 = self.c0.add(cs.ns(|| "a0 + a1"), &self.c1)?;
        let b0_plus_b1 = inverse.c0.add(cs.ns(|| "b0 + b1"), &inverse.c1)?;

        let one = Fp6::<P::Fp6Params>::one();
        let rhs = Self::mul_fp6_by_nonresidue(cs.ns(|| "nr * v1"), &v1)?
            .sub(cs.ns(|| "sub v1"), &v1)?
            .negate(cs.ns(|| "negate it"))?
            .add_constant(cs.ns(|| "add one"), &one)?;
        a0_plus_a1.mul_equals(cs.ns(|| "inv_constraint_2"), &b0_plus_b1, &rhs)?;
        Ok(inverse)
    }

    fn mul_equals<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        result: &Self,
    ) -> Result<(), SynthesisError> {
        // Karatsuba multiplication for Fp2:
        //     v0 = A.c0 * B.c0
        //     v1 = A.c1 * B.c1
        //     result.c0 = v0 + non_residue * v1
        //     result.c1 = (A.c0 + A.c1) * (B.c0 + B.c1) - v0 - v1
        // Enforced with 3 constraints:
        //     A.c1 * B.c1 = v1
        //     A.c0 * B.c0 = result.c0 - non_residue * v1
        //     (A.c0+A.c1)*(B.c0+B.c1) = result.c1 + result.c0 + (1 - non_residue) * v1
        // Reference:
        // "Multiplication and Squaring on Pairing-Friendly Fields"
        // Devegili, OhEigeartaigh, Scott, Dahab
        let mul_cs = &mut cs.ns(|| "mul");

        // Compute v1
        let v1 = self.c1.mul(mul_cs.ns(|| "v1"), &other.c1)?;

        // Perform second check
        let non_residue_times_v1 = Self::mul_fp6_by_nonresidue(mul_cs.ns(|| "nr * v1"), &v1)?;
        let rhs = result
            .c0
            .sub(mul_cs.ns(|| "sub from result.c0"), &non_residue_times_v1)?;
        self.c0.mul_equals(mul_cs.ns(|| "second check"), &other.c0, &rhs)?;

        // Last check
        let a0_plus_a1 = self.c0.add(mul_cs.ns(|| "a0 + a1"), &self.c1)?;
        let b0_plus_b1 = other.c0.add(mul_cs.ns(|| "b0 + b1"), &other.c1)?;
        let one_minus_non_residue_v1 = v1.sub(mul_cs.ns(|| "sub from v1"), &non_residue_times_v1)?;

        let result_c1_plus_result_c0_plus_one_minus_non_residue_v1 = result
            .c1
            .add(mul_cs.ns(|| "c1 + c0"), &result.c0)?
            .add(mul_cs.ns(|| "rest of stuff"), &one_minus_non_residue_v1)?;

        a0_plus_a1.mul_equals(
            mul_cs.ns(|| "third check"),
            &b0_plus_b1,
            &result_c1_plus_result_c0_plus_one_minus_non_residue_v1,
        )?;

        Ok(())
    }

    fn cost_of_mul() -> usize {
        unimplemented!()
    }

    fn cost_of_inv() -> usize {
        Self::cost_of_mul() + <Self as EqGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> PartialEq for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn eq(&self, other: &Self) -> bool {
        self.c0 == other.c0 && self.c1 == other.c1
    }
}

impl<P, F: PrimeField> Eq for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
}

impl<P, F: PrimeField> EqGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
}

impl<P, F: PrimeField> ConditionalEqGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
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
        Ok(())
    }

    fn cost() -> usize {
        2 * <Fp6Gadget<P, F> as ConditionalEqGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> NEqGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn enforce_not_equal<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<(), SynthesisError> {
        self.c0.enforce_not_equal(&mut cs.ns(|| "c0"), &other.c0)?;
        self.c1.enforce_not_equal(&mut cs.ns(|| "c1"), &other.c1)?;
        Ok(())
    }

    fn cost() -> usize {
        2 * <Fp6Gadget<P, F> as NEqGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> ToBitsGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn to_bits<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut c0 = self.c0.to_bits(&mut cs)?;
        let mut c1 = self.c1.to_bits(cs)?;
        c0.append(&mut c1);
        Ok(c0)
    }

    fn to_bits_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut c0 = self.c0.to_bits_strict(&mut cs)?;
        let mut c1 = self.c1.to_bits_strict(cs)?;
        c0.append(&mut c1);
        Ok(c0)
    }
}

impl<P, F: PrimeField> ToBytesGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut c0 = self.c0.to_bytes(cs.ns(|| "c0"))?;
        let mut c1 = self.c1.to_bytes(cs.ns(|| "c1"))?;
        c0.append(&mut c1);
        Ok(c0)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut c0 = self.c0.to_bytes_strict(cs.ns(|| "c0"))?;
        let mut c1 = self.c1.to_bytes_strict(cs.ns(|| "c1"))?;
        c0.append(&mut c1);
        Ok(c0)
    }
}

impl<P, F: PrimeField> Clone for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    fn clone(&self) -> Self {
        Self::new(self.c0.clone(), self.c1.clone())
    }
}

impl<P, F: PrimeField> CondSelectGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn conditionally_select<CS: ConstraintSystem<F>>(
        mut cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError> {
        let c0 = Fp6Gadget::<P, F>::conditionally_select(&mut cs.ns(|| "c0"), cond, &first.c0, &second.c0)?;
        let c1 = Fp6Gadget::<P, F>::conditionally_select(&mut cs.ns(|| "c1"), cond, &first.c1, &second.c1)?;

        Ok(Self::new(c0, c1))
    }

    fn cost() -> usize {
        2 * <Fp6Gadget<P, F> as CondSelectGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> TwoBitLookupGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    type TableConstant = Fp12<P>;

    fn two_bit_lookup<CS: ConstraintSystem<F>>(
        mut cs: CS,
        b: &[Boolean],
        c: &[Self::TableConstant],
    ) -> Result<Self, SynthesisError> {
        let c0s = c.iter().map(|f| f.c0).collect::<Vec<_>>();
        let c1s = c.iter().map(|f| f.c1).collect::<Vec<_>>();
        let c0 = Fp6Gadget::<P, F>::two_bit_lookup(cs.ns(|| "Lookup c0"), b, &c0s)?;
        let c1 = Fp6Gadget::<P, F>::two_bit_lookup(cs.ns(|| "Lookup c1"), b, &c1s)?;
        Ok(Self::new(c0, c1))
    }

    fn cost() -> usize {
        2 * <Fp6Gadget<P, F> as TwoBitLookupGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> ThreeBitCondNegLookupGadget<F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    type TableConstant = Fp12<P>;

    fn three_bit_cond_neg_lookup<CS: ConstraintSystem<F>>(
        mut cs: CS,
        b: &[Boolean],
        b0b1: &Boolean,
        c: &[Self::TableConstant],
    ) -> Result<Self, SynthesisError> {
        let c0s = c.iter().map(|f| f.c0).collect::<Vec<_>>();
        let c1s = c.iter().map(|f| f.c1).collect::<Vec<_>>();
        let c0 = Fp6Gadget::<P, F>::three_bit_cond_neg_lookup(cs.ns(|| "Lookup c0"), b, b0b1, &c0s)?;
        let c1 = Fp6Gadget::<P, F>::three_bit_cond_neg_lookup(cs.ns(|| "Lookup c1"), b, b0b1, &c1s)?;
        Ok(Self::new(c0, c1))
    }

    fn cost() -> usize {
        2 * <Fp6Gadget<P, F> as ThreeBitCondNegLookupGadget<F>>::cost()
    }
}

impl<P, F: PrimeField> AllocGadget<Fp12<P>, F> for Fp12Gadget<P, F>
where
    P: Fp12Parameters,
    <P::Fp6Params as Fp6Parameters>::Fp2Params: Fp2Parameters<Fp = F>,
{
    #[inline]
    fn alloc<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<Fp12<P>>,
    {
        let (c0, c1) = match value_gen() {
            Ok(fe) => {
                let fe = *fe.borrow();
                (Ok(fe.c0), Ok(fe.c1))
            }
            Err(_) => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        let c0 = Fp6Gadget::<P, F>::alloc(&mut cs.ns(|| "c0"), || c0)?;
        let c1 = Fp6Gadget::<P, F>::alloc(&mut cs.ns(|| "c1"), || c1)?;
        Ok(Self::new(c0, c1))
    }

    #[inline]
    fn alloc_input<Fn, T, CS: ConstraintSystem<F>>(mut cs: CS, value_gen: Fn) -> Result<Self, SynthesisError>
    where
        Fn: FnOnce() -> Result<T, SynthesisError>,
        T: Borrow<Fp12<P>>,
    {
        let (c0, c1) = match value_gen() {
            Ok(fe) => {
                let fe = *fe.borrow();
                (Ok(fe.c0), Ok(fe.c1))
            }
            Err(_) => (
                Err(SynthesisError::AssignmentMissing),
                Err(SynthesisError::AssignmentMissing),
            ),
        };

        let c0 = Fp6Gadget::<P, F>::alloc_input(&mut cs.ns(|| "c0"), || c0)?;
        let c1 = Fp6Gadget::<P, F>::alloc_input(&mut cs.ns(|| "c1"), || c1)?;
        Ok(Self::new(c0, c1))
    }
}
