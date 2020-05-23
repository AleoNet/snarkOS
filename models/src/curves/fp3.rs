use crate::curves::{Field, LegendreSymbol, PrimeField, SquareRootField};
use snarkvm_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
};

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::{
    cmp::{Ord, Ordering, PartialOrd},
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

pub trait Fp3Parameters: 'static + Send + Sync {
    type Fp: PrimeField + SquareRootField;

    const NONRESIDUE: Self::Fp;
    const FROBENIUS_COEFF_FP3_C1: [Self::Fp; 3];
    const FROBENIUS_COEFF_FP3_C2: [Self::Fp; 3];
    /// p^3 - 1 = 2^s * t, where t is odd.
    const TWO_ADICITY: u32;
    const T_MINUS_ONE_DIV_TWO: &'static [u64];
    /// t-th power of a quadratic nonresidue in Fp3.
    const QUADRATIC_NONRESIDUE_TO_T: (Self::Fp, Self::Fp, Self::Fp);

    #[inline(always)]
    fn mul_fp_by_nonresidue(fe: &Self::Fp) -> Self::Fp {
        Self::NONRESIDUE * fe
    }
}

#[derive(Derivative)]
#[derivative(
    Default(bound = "P: Fp3Parameters"),
    Hash(bound = "P: Fp3Parameters"),
    Clone(bound = "P: Fp3Parameters"),
    Copy(bound = "P: Fp3Parameters"),
    Debug(bound = "P: Fp3Parameters"),
    PartialEq(bound = "P: Fp3Parameters"),
    Eq(bound = "P: Fp3Parameters")
)]
pub struct Fp3<P: Fp3Parameters> {
    pub c0: P::Fp,
    pub c1: P::Fp,
    pub c2: P::Fp,
    #[derivative(Debug = "ignore")]
    #[doc(hidden)]
    pub _parameters: PhantomData<P>,
}

impl<P: Fp3Parameters> Fp3<P> {
    pub fn new(c0: P::Fp, c1: P::Fp, c2: P::Fp) -> Self {
        Fp3 {
            c0,
            c1,
            c2,
            _parameters: PhantomData,
        }
    }

    pub fn mul_assign_by_fp(&mut self, value: &P::Fp) {
        self.c0.mul_assign(value);
        self.c1.mul_assign(value);
        self.c2.mul_assign(value);
    }

    // Calculate the norm of an element with respect to the base field Fp.
    pub fn norm(&self) -> P::Fp {
        let mut self_to_p = *self;
        self_to_p.frobenius_map(1);
        let mut self_to_p2 = *self;
        self_to_p2.frobenius_map(2);
        self_to_p *= &(self_to_p2 * self);
        assert!(self_to_p.c1.is_zero() && self_to_p.c2.is_zero());
        self_to_p.c0
    }

    // Returns the value of QNR^T.
    #[inline]
    pub fn qnr_to_t() -> Self {
        Self::new(
            P::QUADRATIC_NONRESIDUE_TO_T.0,
            P::QUADRATIC_NONRESIDUE_TO_T.1,
            P::QUADRATIC_NONRESIDUE_TO_T.2,
        )
    }
}

impl<P: Fp3Parameters> Field for Fp3<P> {
    fn zero() -> Self {
        Fp3 {
            c0: P::Fp::zero(),
            c1: P::Fp::zero(),
            c2: P::Fp::zero(),
            _parameters: PhantomData,
        }
    }

    fn is_zero(&self) -> bool {
        self.c0.is_zero() && self.c1.is_zero() && self.c2.is_zero()
    }

    fn one() -> Self {
        Fp3 {
            c0: P::Fp::one(),
            c1: P::Fp::zero(),
            c2: P::Fp::zero(),
            _parameters: PhantomData,
        }
    }

    fn is_one(&self) -> bool {
        self.c0.is_one() && self.c1.is_zero() && self.c2.is_zero()
    }

    #[inline]
    fn characteristic<'a>() -> &'a [u64] {
        P::Fp::characteristic()
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
        // Devegili OhEig Scott Dahab --- Multiplication and Squaring on
        // AbstractPairing-Friendly
        // Fields.pdf; Section 4 (CH-SQR2)
        let a = self.c0.clone();
        let b = self.c1.clone();
        let c = self.c2.clone();

        let s0 = a.square();
        let ab = a * &b;
        let s1 = ab + &ab;
        let s2 = (a - &b + &c).square();
        let bc = b * &c;
        let s3 = bc + &bc;
        let s4 = c.square();

        self.c0 = s0 + &P::mul_fp_by_nonresidue(&s3);
        self.c1 = s1 + &P::mul_fp_by_nonresidue(&s4);
        self.c2 = s1 + &s2 + &s3 - &s0 - &s4;
        self
    }

    fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            // From "High-Speed Software Implementation of the Optimal Ate AbstractPairing
            // over
            // Barreto-Naehrig Curves"; Algorithm 17
            let t0 = self.c0.square();
            let t1 = self.c1.square();
            let t2 = self.c2.square();
            let mut t3 = self.c0.clone();
            t3.mul_assign(&self.c1);
            let mut t4 = self.c0.clone();
            t4.mul_assign(&self.c2);
            let mut t5 = self.c1.clone();
            t5.mul_assign(&self.c2);
            let n5 = P::mul_fp_by_nonresidue(&t5);

            let mut s0 = t0.clone();
            s0.sub_assign(&n5);
            let mut s1 = P::mul_fp_by_nonresidue(&t2);
            s1.sub_assign(&t3);
            let mut s2 = t1.clone();
            s2.sub_assign(&t4); // typo in paper referenced above. should be "-" as per Scott, but is "*"

            let mut a1 = self.c2.clone();
            a1.mul_assign(&s1);
            let mut a2 = self.c1.clone();
            a2.mul_assign(&s2);
            let mut a3 = a1.clone();
            a3.add_assign(&a2);
            a3 = P::mul_fp_by_nonresidue(&a3);
            let mut t6 = self.c0.clone();
            t6.mul_assign(&s0);
            t6.add_assign(&a3);
            t6.inverse_in_place();

            let mut c0 = t6.clone();
            c0.mul_assign(&s0);
            let mut c1 = t6.clone();
            c1.mul_assign(&s1);
            let mut c2 = t6.clone();
            c2.mul_assign(&s2);

            Some(Self::new(c0, c1, c2))
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
        self.c1.mul_assign(&P::FROBENIUS_COEFF_FP3_C1[power % 3]);
        self.c2.mul_assign(&P::FROBENIUS_COEFF_FP3_C2[power % 3]);
    }
}

impl<P: Fp3Parameters> SquareRootField for Fp3<P> {
    /// Returns the Legendre symbol.
    fn legendre(&self) -> LegendreSymbol {
        self.norm().legendre()
    }

    /// Returns the square root of self, if it exists.
    fn sqrt(&self) -> Option<Self> {
        sqrt_impl!(Self, P, self)
    }

    /// Sets `self` to be the square root of `self`, if it exists.
    fn sqrt_in_place(&mut self) -> Option<&mut Self> {
        (*self).sqrt().map(|sqrt| {
            *self = sqrt;
            self
        })
    }
}

/// `Fp3` elements are ordered lexicographically.
impl<P: Fp3Parameters> Ord for Fp3<P> {
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

impl<P: Fp3Parameters> PartialOrd for Fp3<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Fp3Parameters> From<u128> for Fp3<P> {
    fn from(other: u128) -> Self {
        let fe: P::Fp = other.into();
        Self::new(fe, P::Fp::zero(), P::Fp::zero())
    }
}

impl<P: Fp3Parameters> From<u64> for Fp3<P> {
    fn from(other: u64) -> Self {
        let fe: P::Fp = other.into();
        Self::new(fe, P::Fp::zero(), P::Fp::zero())
    }
}

impl<P: Fp3Parameters> From<u32> for Fp3<P> {
    fn from(other: u32) -> Self {
        let fe: P::Fp = other.into();
        Self::new(fe, P::Fp::zero(), P::Fp::zero())
    }
}

impl<P: Fp3Parameters> From<u16> for Fp3<P> {
    fn from(other: u16) -> Self {
        let fe: P::Fp = other.into();
        Self::new(fe, P::Fp::zero(), P::Fp::zero())
    }
}

impl<P: Fp3Parameters> From<u8> for Fp3<P> {
    fn from(other: u8) -> Self {
        let fe: P::Fp = other.into();
        Self::new(fe, P::Fp::zero(), P::Fp::zero())
    }
}

impl<P: Fp3Parameters> ToBytes for Fp3<P> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.c0.write(&mut writer)?;
        self.c1.write(&mut writer)?;
        self.c2.write(writer)
    }
}

impl<P: Fp3Parameters> FromBytes for Fp3<P> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let c0 = P::Fp::read(&mut reader)?;
        let c1 = P::Fp::read(&mut reader)?;
        let c2 = P::Fp::read(reader)?;
        Ok(Fp3::new(c0, c1, c2))
    }
}

impl<P: Fp3Parameters> Neg for Fp3<P> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        let mut res = self.clone();
        res.c0 = res.c0.neg();
        res.c1 = res.c1.neg();
        res.c2 = res.c2.neg();
        res
    }
}

impl<P: Fp3Parameters> Distribution<Fp3<P>> for Standard {
    #[inline]
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Fp3<P> {
        Fp3::new(UniformRand::rand(rng), UniformRand::rand(rng), UniformRand::rand(rng))
    }
}

impl<'a, P: Fp3Parameters> Add<&'a Fp3<P>> for Fp3<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self;
        result.add_assign(&other);
        result
    }
}

impl<'a, P: Fp3Parameters> Sub<&'a Fp3<P>> for Fp3<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self;
        result.sub_assign(&other);
        result
    }
}

impl<'a, P: Fp3Parameters> Mul<&'a Fp3<P>> for Fp3<P> {
    type Output = Self;

    #[inline]
    fn mul(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(&other);
        result
    }
}

impl<'a, P: Fp3Parameters> Div<&'a Fp3<P>> for Fp3<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp3Parameters> AddAssign<&'a Self> for Fp3<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        self.c0.add_assign(&other.c0);
        self.c1.add_assign(&other.c1);
        self.c2.add_assign(&other.c2);
    }
}

impl<'a, P: Fp3Parameters> SubAssign<&'a Self> for Fp3<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        self.c0.sub_assign(&other.c0);
        self.c1.sub_assign(&other.c1);
        self.c2.sub_assign(&other.c2);
    }
}

impl<'a, P: Fp3Parameters> MulAssign<&'a Self> for Fp3<P> {
    #[inline]
    fn mul_assign(&mut self, other: &Self) {
        // Devegili OhEig Scott Dahab --- Multiplication and Squaring on
        // AbstractPairing-Friendly
        // Fields.pdf; Section 4 (Karatsuba)

        let a = other.c0;
        let b = other.c1;
        let c = other.c2;

        let d = self.c0;
        let e = self.c1;
        let f = self.c2;

        let ad = d * &a;
        let be = e * &b;
        let cf = f * &c;

        let x = (e + &f) * &(b + &c) - &be - &cf;
        let y = (d + &e) * &(a + &b) - &ad - &be;
        let z = (d + &f) * &(a + &c) - &ad + &be - &cf;

        self.c0 = ad + &P::mul_fp_by_nonresidue(&x);
        self.c1 = y + &P::mul_fp_by_nonresidue(&cf);
        self.c2 = z;
    }
}

impl<'a, P: Fp3Parameters> DivAssign<&'a Self> for Fp3<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}

impl<P: Fp3Parameters> FromStr for Fp3<P> {
    type Err = ();

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        s = s.trim();
        if s.is_empty() {
            println!("is empty");
            return Err(());
        }
        if s.len() < 3 {
            println!("len is less than 3");
            return Err(());
        }
        if !(s.starts_with('[') && s.ends_with(']')) {
            println!("doesn't start and end with square brackets");
            return Err(());
        }
        let mut point = Vec::new();
        for substr in s.split(|c| c == '[' || c == ']' || c == ',' || c == ' ') {
            if !substr.is_empty() {
                let coord = P::Fp::from_str(substr).map_err(|_| ())?;
                point.push(coord);
            }
        }
        if point.len() != 3 {
            println!("not enough points");
            return Err(());
        }
        let point = Fp3::new(point[0], point[1], point[2]);
        Ok(point)
    }
}

impl<P: Fp3Parameters> ::std::fmt::Display for Fp3<P> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        write!(f, "Fp3({}, {}, {})", self.c0, self.c1, self.c2)
    }
}
