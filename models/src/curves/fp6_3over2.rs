use crate::curves::{Field, Fp2, Fp2Parameters};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    serialize::*,
};

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub trait Fp6Parameters: 'static + Send + Sync + Copy {
    type Fp2Params: Fp2Parameters;

    /// Coefficients for the Frobenius automorphism.
    const FROBENIUS_COEFF_FP6_C1: [Fp2<Self::Fp2Params>; 6];
    const FROBENIUS_COEFF_FP6_C2: [Fp2<Self::Fp2Params>; 6];

    const NONRESIDUE: Fp2<Self::Fp2Params>;

    #[inline(always)]
    fn mul_fp2_by_nonresidue(fe: &Fp2<Self::Fp2Params>) -> Fp2<Self::Fp2Params> {
        Self::NONRESIDUE * fe
    }
}

/// An element of Fp6, represented by c0 + c1 * v + c2 * v^(2).
#[derive(Derivative, Serialize, Deserialize)]
#[derivative(
    Default(bound = "P: Fp6Parameters"),
    Hash(bound = "P: Fp6Parameters"),
    Clone(bound = "P: Fp6Parameters"),
    Copy(bound = "P: Fp6Parameters"),
    Debug(bound = "P: Fp6Parameters"),
    PartialEq(bound = "P: Fp6Parameters"),
    Eq(bound = "P: Fp6Parameters")
)]
pub struct Fp6<P: Fp6Parameters> {
    pub c0: Fp2<P::Fp2Params>,
    pub c1: Fp2<P::Fp2Params>,
    pub c2: Fp2<P::Fp2Params>,
    #[derivative(Debug = "ignore")]
    #[doc(hidden)]
    pub params: PhantomData<P>,
}

impl<P: Fp6Parameters> Fp6<P> {
    pub fn new(c0: Fp2<P::Fp2Params>, c1: Fp2<P::Fp2Params>, c2: Fp2<P::Fp2Params>) -> Self {
        Self {
            c0,
            c1,
            c2,
            params: PhantomData,
        }
    }

    pub fn mul_by_fp(&mut self, element: &<P::Fp2Params as Fp2Parameters>::Fp) {
        self.c0.mul_by_fp(&element);
        self.c1.mul_by_fp(&element);
        self.c2.mul_by_fp(&element);
    }

    pub fn mul_by_fp2(&mut self, element: &Fp2<P::Fp2Params>) {
        self.c0.mul_assign(&element);
        self.c1.mul_assign(&element);
        self.c2.mul_assign(&element);
    }

    pub fn mul_by_1(&mut self, c1: &Fp2<P::Fp2Params>) {
        let mut b_b = self.c1;
        b_b.mul_assign(c1);

        let mut t1 = *c1;
        {
            let mut tmp = self.c1;
            tmp.add_assign(&self.c2);

            t1.mul_assign(&tmp);
            t1.sub_assign(&b_b);
            t1 = P::mul_fp2_by_nonresidue(&t1);
        }

        let mut t2 = *c1;
        {
            let mut tmp = self.c0;
            tmp.add_assign(&self.c1);

            t2.mul_assign(&tmp);
            t2.sub_assign(&b_b);
        }

        self.c0 = t1;
        self.c1 = t2;
        self.c2 = b_b;
    }

    pub fn mul_by_01(&mut self, c0: &Fp2<P::Fp2Params>, c1: &Fp2<P::Fp2Params>) {
        let mut a_a = self.c0;
        let mut b_b = self.c1;
        a_a.mul_assign(c0);
        b_b.mul_assign(c1);

        let mut t1 = *c1;
        {
            let mut tmp = self.c1;
            tmp.add_assign(&self.c2);

            t1.mul_assign(&tmp);
            t1.sub_assign(&b_b);
            t1 = P::mul_fp2_by_nonresidue(&t1);
            t1.add_assign(&a_a);
        }

        let mut t3 = *c0;
        {
            let mut tmp = self.c0;
            tmp.add_assign(&self.c2);

            t3.mul_assign(&tmp);
            t3.sub_assign(&a_a);
            t3.add_assign(&b_b);
        }

        let mut t2 = *c0;
        t2.add_assign(c1);
        {
            let mut tmp = self.c0;
            tmp.add_assign(&self.c1);

            t2.mul_assign(&tmp);
            t2.sub_assign(&a_a);
            t2.sub_assign(&b_b);
        }

        self.c0 = t1;
        self.c1 = t2;
        self.c2 = t3;
    }
}

impl<P: Fp6Parameters> Field for Fp6<P> {
    fn zero() -> Self {
        Self::new(Fp2::zero(), Fp2::zero(), Fp2::zero())
    }

    fn is_zero(&self) -> bool {
        self.c0.is_zero() && self.c1.is_zero() && self.c2.is_zero()
    }

    fn one() -> Self {
        Self::new(Fp2::one(), Fp2::zero(), Fp2::zero())
    }

    fn is_one(&self) -> bool {
        self.c0.is_one() && self.c1.is_zero() && self.c2.is_zero()
    }

    #[inline]
    fn characteristic<'a>() -> &'a [u64] {
        Fp2::<P::Fp2Params>::characteristic()
    }

    fn double(&self) -> Self {
        let mut result = self.clone();
        result.double_in_place();
        result
    }

    fn double_in_place(&mut self) -> &mut Self {
        self.c0.double_in_place();
        self.c1.double_in_place();
        self.c2.double_in_place();
        self
    }

    fn square(&self) -> Self {
        let mut result = self.clone();
        result.square_in_place();
        result
    }

    fn square_in_place(&mut self) -> &mut Self {
        let s0 = self.c0.square();
        let s1 = (self.c0 * &self.c1).double();
        let s2 = (self.c0 - &self.c1 + &self.c2).square();
        let s3 = (self.c1 * &self.c2).double();
        let s4 = self.c2.square();

        self.c0 = s0 + &P::mul_fp2_by_nonresidue(&s3);
        self.c1 = s1 + &P::mul_fp2_by_nonresidue(&s4);
        self.c2 = s1 + &s2 + &s3 - &s0 - &s4;

        self
    }

    fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            let mut c0 = self.c2;
            c0 = P::mul_fp2_by_nonresidue(&c0);
            c0.mul_assign(&self.c1);
            c0 = c0.neg();
            {
                let mut c0s = self.c0;
                c0s.square_in_place();
                c0.add_assign(&c0s);
            }
            let mut c1 = self.c2;
            c1.square_in_place();
            c1 = P::mul_fp2_by_nonresidue(&c1);
            {
                let mut c01 = self.c0;
                c01.mul_assign(&self.c1);
                c1.sub_assign(&c01);
            }
            let mut c2 = self.c1;
            c2.square_in_place();
            {
                let mut c02 = self.c0;
                c02.mul_assign(&self.c2);
                c2.sub_assign(&c02);
            }

            let mut tmp1 = self.c2;
            tmp1.mul_assign(&c1);
            let mut tmp2 = self.c1;
            tmp2.mul_assign(&c2);
            tmp1.add_assign(&tmp2);
            tmp1 = P::mul_fp2_by_nonresidue(&tmp1);
            tmp2 = self.c0;
            tmp2.mul_assign(&c0);
            tmp1.add_assign(&tmp2);

            match tmp1.inverse() {
                Some(t) => Some(Self::new(t * &c0, t * &c1, t * &c2)),
                None => None,
            }
        }
    }

    fn inverse_in_place(&mut self) -> Option<&mut Self> {
        if let Some(inverse) = self.inverse() {
            *self = inverse;
            Some(self)
        } else {
            None
        }
    }

    fn frobenius_map(&mut self, power: usize) {
        self.c0.frobenius_map(power);
        self.c1.frobenius_map(power);
        self.c2.frobenius_map(power);

        self.c1.mul_assign(&P::FROBENIUS_COEFF_FP6_C1[power % 6]);
        self.c2.mul_assign(&P::FROBENIUS_COEFF_FP6_C2[power % 6]);
    }
}

impl<P: Fp6Parameters> std::fmt::Display for Fp6<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fq6_3over2({} + {} * v, {} * v^2)", self.c0, self.c1, self.c2)
    }
}

impl<P: Fp6Parameters> Distribution<Fp6<P>> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp6<P> {
        Fp6::new(UniformRand::rand(rng), UniformRand::rand(rng), UniformRand::rand(rng))
    }
}

impl<P: Fp6Parameters> Neg for Fp6<P> {
    type Output = Self;

    #[inline]
    #[must_use]
    fn neg(self) -> Self {
        let mut copy = Self::zero();
        copy.c0 = self.c0.neg();
        copy.c1 = self.c1.neg();
        copy.c2 = self.c2.neg();
        copy
    }
}

impl<'a, P: Fp6Parameters> Add<&'a Self> for Fp6<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self;
        result.add_assign(&other);
        result
    }
}

impl<'a, P: Fp6Parameters> Sub<&'a Self> for Fp6<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self;
        result.sub_assign(&other);
        result
    }
}

impl<'a, P: Fp6Parameters> Mul<&'a Self> for Fp6<P> {
    type Output = Self;

    #[inline]
    fn mul(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(&other);
        result
    }
}

impl<'a, P: Fp6Parameters> Div<&'a Self> for Fp6<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp6Parameters> AddAssign<&'a Self> for Fp6<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        self.c0 += &other.c0;
        self.c1 += &other.c1;
        self.c2 += &other.c2;
    }
}

impl<'a, P: Fp6Parameters> SubAssign<&'a Self> for Fp6<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        self.c0 -= &other.c0;
        self.c1 -= &other.c1;
        self.c2 -= &other.c2;
    }
}

impl<'a, P: Fp6Parameters> MulAssign<&'a Self> for Fp6<P> {
    #[inline]
    fn mul_assign(&mut self, other: &Self) {
        let v0 = self.c0 * &other.c0;
        let v1 = self.c1 * &other.c1;
        let v2 = self.c2 * &other.c2;

        let c0 = P::mul_fp2_by_nonresidue(&((self.c1 + &self.c2) * &(other.c1 + &other.c2) - &v1 - &v2)) + &v0;
        let c1 = (self.c0 + &self.c1) * &(other.c0 + &other.c1) - &v0 - &v1 + &P::mul_fp2_by_nonresidue(&v2);
        let c2 = (self.c0 + &self.c2) * &(other.c0 + &other.c2) - &v0 - &v2 + &v1;

        self.c0 = c0;
        self.c1 = c1;
        self.c2 = c2;
    }
}

impl<'a, P: Fp6Parameters> DivAssign<&'a Self> for Fp6<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}

impl<'a, P: Fp6Parameters> From<&'a [bool]> for Fp6<P> {
    fn from(_bits: &[bool]) -> Self {
        unimplemented!()
    }
}

/// `Fp3` elements are ordered lexicographically.
impl<P: Fp6Parameters> Ord for Fp6<P> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        let c2_cmp = self.c2.cmp(&other.c2);
        let c1_cmp = self.c1.cmp(&other.c1);
        let c0_cmp = self.c0.cmp(&other.c0);
        if c2_cmp == Ordering::Equal {
            if c1_cmp == Ordering::Equal { c0_cmp } else { c1_cmp }
        } else {
            c2_cmp
        }
    }
}

impl<P: Fp6Parameters> PartialOrd for Fp6<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Fp6Parameters> From<u128> for Fp6<P> {
    fn from(other: u128) -> Self {
        Self::new(other.into(), Fp2::zero(), Fp2::zero())
    }
}

impl<P: Fp6Parameters> From<u64> for Fp6<P> {
    fn from(other: u64) -> Self {
        Self::new(other.into(), Fp2::zero(), Fp2::zero())
    }
}

impl<P: Fp6Parameters> From<u32> for Fp6<P> {
    fn from(other: u32) -> Self {
        Self::new(other.into(), Fp2::zero(), Fp2::zero())
    }
}

impl<P: Fp6Parameters> From<u16> for Fp6<P> {
    fn from(other: u16) -> Self {
        Self::new(other.into(), Fp2::zero(), Fp2::zero())
    }
}

impl<P: Fp6Parameters> From<u8> for Fp6<P> {
    fn from(other: u8) -> Self {
        Self::new(other.into(), Fp2::zero(), Fp2::zero())
    }
}

impl<P: Fp6Parameters> ToBytes for Fp6<P> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.c0.write(&mut writer)?;
        self.c1.write(&mut writer)?;
        self.c2.write(&mut writer)
    }
}

impl<P: Fp6Parameters> FromBytes for Fp6<P> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let c0 = Fp2::read(&mut reader)?;
        let c1 = Fp2::read(&mut reader)?;
        let c2 = Fp2::read(&mut reader)?;
        Ok(Fp6::new(c0, c1, c2))
    }
}

impl<P: Fp6Parameters> CanonicalSerializeWithFlags for Fp6<P> {
    #[inline]
    fn serialize_with_flags<W: Write, F: Flags>(&self, writer: &mut W, flags: F) -> Result<(), SerializationError> {
        CanonicalSerialize::serialize(&self.c0, writer)?;
        CanonicalSerialize::serialize(&self.c1, writer)?;
        self.c2.serialize_with_flags(writer, flags)?;
        Ok(())
    }
}

impl<P: Fp6Parameters> CanonicalSerialize for Fp6<P> {
    #[inline]
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<(), SerializationError> {
        self.serialize_with_flags(writer, EmptyFlags)
    }

    #[inline]
    fn serialized_size(&self) -> usize {
        Self::SERIALIZED_SIZE
    }
}

impl<P: Fp6Parameters> ConstantSerializedSize for Fp6<P> {
    const SERIALIZED_SIZE: usize = 3 * <Fp2<P::Fp2Params> as ConstantSerializedSize>::SERIALIZED_SIZE;
    const UNCOMPRESSED_SIZE: usize = Self::SERIALIZED_SIZE;
}

impl<P: Fp6Parameters> CanonicalDeserializeWithFlags for Fp6<P> {
    #[inline]
    fn deserialize_with_flags<R: Read, F: Flags>(reader: &mut R) -> Result<(Self, F), SerializationError> {
        let c0 = CanonicalDeserialize::deserialize(reader)?;
        let c1 = CanonicalDeserialize::deserialize(reader)?;
        let (c2, flags) = Fp2::deserialize_with_flags(reader)?;
        Ok((Fp6::new(c0, c1, c2), flags))
    }
}

impl<P: Fp6Parameters> CanonicalDeserialize for Fp6<P> {
    #[inline]
    fn deserialize<R: Read>(reader: &mut R) -> Result<Self, SerializationError> {
        let c0 = CanonicalDeserialize::deserialize(reader)?;
        let c1 = CanonicalDeserialize::deserialize(reader)?;
        let c2 = CanonicalDeserialize::deserialize(reader)?;
        Ok(Fp6::new(c0, c1, c2))
    }
}
