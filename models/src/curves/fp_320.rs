use crate::curves::{Field, FpParameters, LegendreSymbol, PrimeField, SquareRootField};
use snarkos_utilities::{
    biginteger::{arithmetic as fa, BigInteger as _BigInteger, BigInteger320 as BigInteger},
    bytes::{FromBytes, ToBytes},
};

use std::{
    cmp::{Ord, Ordering, PartialOrd},
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

pub trait Fp320Parameters: FpParameters<BigInt = BigInteger> {}

#[derive(Derivative)]
#[derivative(
    Default(bound = "P: Fp320Parameters"),
    Hash(bound = "P: Fp320Parameters"),
    Clone(bound = "P: Fp320Parameters"),
    Copy(bound = "P: Fp320Parameters"),
    Debug(bound = "P: Fp320Parameters"),
    PartialEq(bound = "P: Fp320Parameters"),
    Eq(bound = "P: Fp320Parameters")
)]
pub struct Fp320<P: Fp320Parameters>(
    pub BigInteger,
    #[derivative(Debug = "ignore")]
    #[doc(hidden)]
    pub PhantomData<P>,
);

impl<P: Fp320Parameters> Fp320<P> {
    #[inline]
    pub fn new(element: BigInteger) -> Self {
        Fp320::<P>(element, PhantomData)
    }

    #[inline]
    fn is_valid(&self) -> bool {
        self.0 < P::MODULUS
    }

    #[inline]
    fn reduce(&mut self) {
        if !self.is_valid() {
            self.0.sub_noborrow(&P::MODULUS);
        }
    }

    #[inline]
    fn mont_reduce(
        &mut self,
        r0: u64,
        mut r1: u64,
        mut r2: u64,
        mut r3: u64,
        mut r4: u64,
        mut r5: u64,
        mut r6: u64,
        mut r7: u64,
        mut r8: u64,
        mut r9: u64,
    ) {
        // The Montgomery reduction here is based on Algorithm 14.32 in
        // Handbook of Applied Cryptography
        // <http://cacr.uwaterloo.ca/hac/about/chap14.pdf>.

        let k = r0.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r0, k, P::MODULUS.0[0], &mut carry);
        r1 = fa::mac_with_carry(r1, k, P::MODULUS.0[1], &mut carry);
        r2 = fa::mac_with_carry(r2, k, P::MODULUS.0[2], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[3], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[4], &mut carry);
        r5 = fa::adc(r5, 0, &mut carry);
        let carry2 = carry;
        let k = r1.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r1, k, P::MODULUS.0[0], &mut carry);
        r2 = fa::mac_with_carry(r2, k, P::MODULUS.0[1], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[2], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[3], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[4], &mut carry);
        r6 = fa::adc(r6, carry2, &mut carry);
        let carry2 = carry;
        let k = r2.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r2, k, P::MODULUS.0[0], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[1], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[2], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[3], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[4], &mut carry);
        r7 = fa::adc(r7, carry2, &mut carry);
        let carry2 = carry;
        let k = r3.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r3, k, P::MODULUS.0[0], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[1], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[2], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[3], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[4], &mut carry);
        r8 = fa::adc(r8, carry2, &mut carry);
        let carry2 = carry;
        let k = r4.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r4, k, P::MODULUS.0[0], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[1], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[2], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[3], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[4], &mut carry);
        r9 = fa::adc(r9, carry2, &mut carry);
        (self.0).0[0] = r5;
        (self.0).0[1] = r6;
        (self.0).0[2] = r7;
        (self.0).0[3] = r8;
        (self.0).0[4] = r9;
        self.reduce();
    }
}

impl<P: Fp320Parameters> Field for Fp320<P> {
    #[inline]
    fn zero() -> Self {
        Fp320::<P>(BigInteger::from(0), PhantomData)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[inline]
    fn double(&self) -> Self {
        let mut temp = *self;
        temp.double_in_place();
        temp
    }

    #[inline]
    fn double_in_place(&mut self) -> &mut Self {
        // This cannot exceed the backing capacity.
        self.0.mul2();
        // However, it may need to be reduced.
        self.reduce();
        self
    }

    #[inline]
    fn one() -> Self {
        Fp320::<P>(P::R, PhantomData)
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.0 == P::R
    }

    #[inline]
    fn characteristic<'a>() -> &'a [u64] {
        P::MODULUS.as_ref()
    }

    #[inline]
    fn square(&self) -> Self {
        let mut temp = self.clone();
        temp.square_in_place();
        temp
    }

    #[inline]
    fn square_in_place(&mut self) -> &mut Self {
        let mut carry = 0;
        let r1 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[1], &mut carry);
        let r2 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[2], &mut carry);
        let r3 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[3], &mut carry);
        let r4 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[4], &mut carry);
        let r5 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[1], (self.0).0[2], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1], (self.0).0[3], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1], (self.0).0[4], &mut carry);
        let r6 = carry;
        let mut carry = 0;
        let r5 = fa::mac_with_carry(r5, (self.0).0[2], (self.0).0[3], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2], (self.0).0[4], &mut carry);
        let r7 = carry;
        let mut carry = 0;
        let r7 = fa::mac_with_carry(r7, (self.0).0[3], (self.0).0[4], &mut carry);
        let r8 = carry;

        let r9 = r8 >> 63;
        let r8 = (r8 << 1) | (r7 >> 63);
        let r7 = (r7 << 1) | (r6 >> 63);
        let r6 = (r6 << 1) | (r5 >> 63);
        let r5 = (r5 << 1) | (r4 >> 63);
        let r4 = (r4 << 1) | (r3 >> 63);
        let r3 = (r3 << 1) | (r2 >> 63);
        let r2 = (r2 << 1) | (r1 >> 63);
        let r1 = r1 << 1;

        let mut carry = 0;
        let r0 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[0], &mut carry);
        let r1 = fa::adc(r1, 0, &mut carry);
        let r2 = fa::mac_with_carry(r2, (self.0).0[1], (self.0).0[1], &mut carry);
        let r3 = fa::adc(r3, 0, &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[2], (self.0).0[2], &mut carry);
        let r5 = fa::adc(r5, 0, &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[3], (self.0).0[3], &mut carry);
        let r7 = fa::adc(r7, 0, &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[4], (self.0).0[4], &mut carry);
        let r9 = fa::adc(r9, 0, &mut carry);
        self.mont_reduce(r0, r1, r2, r3, r4, r5, r6, r7, r8, r9);
        self
    }

    #[inline]
    fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            // Guajardo Kumar Paar Pelzl
            // Efficient Software-Implementation of Finite Fields with Applications to
            // Cryptography
            // Algorithm 16 (BEA for Inversion in Fp)

            let one = BigInteger::from(1);

            let mut u = self.0;
            let mut v = P::MODULUS;
            let mut b = Fp320::<P>(P::R2, PhantomData); // Avoids unnecessary reduction step.
            let mut c = Self::zero();

            while u != one && v != one {
                while u.is_even() {
                    u.div2();

                    if b.0.is_even() {
                        b.0.div2();
                    } else {
                        b.0.add_nocarry(&P::MODULUS);
                        b.0.div2();
                    }
                }

                while v.is_even() {
                    v.div2();

                    if c.0.is_even() {
                        c.0.div2();
                    } else {
                        c.0.add_nocarry(&P::MODULUS);
                        c.0.div2();
                    }
                }

                if v < u {
                    u.sub_noborrow(&v);
                    b.sub_assign(&c);
                } else {
                    v.sub_noborrow(&u);
                    c.sub_assign(&b);
                }
            }

            if u == one { Some(b) } else { Some(c) }
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

    #[inline]
    fn frobenius_map(&mut self, _: usize) {
        // No-op: No effect in a prime field.
    }
}

impl<P: Fp320Parameters> PrimeField for Fp320<P> {
    type BigInt = BigInteger;
    type Params = P;

    #[inline]
    fn from_repr(r: BigInteger) -> Self {
        let mut r = Fp320(r, PhantomData);
        if r.is_valid() {
            r.mul_assign(&Fp320(P::R2, PhantomData));
            r
        } else {
            Self::zero()
        }
    }

    #[inline]
    fn into_repr(&self) -> BigInteger {
        let mut r = *self;
        r.mont_reduce(
            (self.0).0[0],
            (self.0).0[1],
            (self.0).0[2],
            (self.0).0[3],
            (self.0).0[4],
            0,
            0,
            0,
            0,
            0,
        );
        r.0
    }

    #[inline]
    fn from_repr_raw(r: BigInteger) -> Self {
        let r = Fp320(r, PhantomData);
        if r.is_valid() { r } else { Self::zero() }
    }

    #[inline]
    fn into_repr_raw(&self) -> BigInteger {
        let r = *self;
        r.0
    }

    #[inline]
    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        let mut result = Self::zero();
        if result.0.read_le((&bytes[..]).by_ref()).is_ok() {
            result.0.as_mut()[4] &= 0xffffffffffffffff >> P::REPR_SHAVE_BITS;
            if result.is_valid() { Some(result) } else { None }
        } else {
            None
        }
    }

    #[inline]
    fn multiplicative_generator() -> Self {
        Fp320::<P>(P::GENERATOR, PhantomData)
    }

    #[inline]
    fn root_of_unity() -> Self {
        Fp320::<P>(P::ROOT_OF_UNITY, PhantomData)
    }
}

impl<P: Fp320Parameters> SquareRootField for Fp320<P> {
    #[inline]
    fn legendre(&self) -> LegendreSymbol {
        use crate::curves::LegendreSymbol::*;

        // s = self^((MODULUS - 1) // 2)
        let s = self.pow(P::MODULUS_MINUS_ONE_DIV_TWO);
        if s.is_zero() {
            Zero
        } else if s.is_one() {
            QuadraticResidue
        } else {
            QuadraticNonResidue
        }
    }

    #[inline]
    fn sqrt(&self) -> Option<Self> {
        sqrt_impl!(Self, P, self)
    }

    fn sqrt_in_place(&mut self) -> Option<&mut Self> {
        if let Some(sqrt) = self.sqrt() {
            *self = sqrt;
            Some(self)
        } else {
            None
        }
    }
}

/// `Fp` elements are ordered lexicographically.
impl<P: Fp320Parameters> Ord for Fp320<P> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.into_repr().cmp(&other.into_repr())
    }
}

impl<P: Fp320Parameters> PartialOrd for Fp320<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl_prime_field_from_int!(Fp320, u128, Fp320Parameters);
impl_prime_field_from_int!(Fp320, u64, Fp320Parameters);
impl_prime_field_from_int!(Fp320, u32, Fp320Parameters);
impl_prime_field_from_int!(Fp320, u16, Fp320Parameters);
impl_prime_field_from_int!(Fp320, u8, Fp320Parameters);

impl_prime_field_standard_sample!(Fp320, Fp320Parameters);

impl<P: Fp320Parameters> ToBytes for Fp320<P> {
    #[inline]
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.into_repr().write(writer)
    }
}

impl<P: Fp320Parameters> FromBytes for Fp320<P> {
    #[inline]
    fn read<R: Read>(reader: R) -> IoResult<Self> {
        BigInteger::read(reader).map(Fp320::from_repr)
    }
}

impl<P: Fp320Parameters> FromStr for Fp320<P> {
    type Err = ();

    /// Interpret a string of numbers as a (congruent) prime field element.
    /// Does not accept unnecessary leading zeroes or a blank string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            println!("Is empty!");
            return Err(());
        }

        if s == "0" {
            return Ok(Self::zero());
        }

        let mut res = Self::zero();

        let ten = Self::from_repr(<Self as PrimeField>::BigInt::from(10));

        let mut first_digit = true;

        for c in s.chars() {
            match c.to_digit(10) {
                Some(c) => {
                    if first_digit {
                        if c == 0 {
                            return Err(());
                        }

                        first_digit = false;
                    }

                    res.mul_assign(&ten);
                    res.add_assign(&Self::from_repr(<Self as PrimeField>::BigInt::from(u64::from(c))));
                }
                None => {
                    println!("Not valid digit!");
                    return Err(());
                }
            }
        }
        if !res.is_valid() { Err(()) } else { Ok(res) }
    }
}

impl<P: Fp320Parameters> Display for Fp320<P> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Fp320({})", self.into_repr())
    }
}

impl<P: Fp320Parameters> Neg for Fp320<P> {
    type Output = Self;

    #[inline]
    #[must_use]
    fn neg(self) -> Self {
        if !self.is_zero() {
            let mut tmp = P::MODULUS.clone();
            tmp.sub_noborrow(&self.0);
            Fp320::<P>(tmp, PhantomData)
        } else {
            self
        }
    }
}

impl<'a, P: Fp320Parameters> Add<&'a Fp320<P>> for Fp320<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.add_assign(other);
        result
    }
}

impl<'a, P: Fp320Parameters> Sub<&'a Fp320<P>> for Fp320<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.sub_assign(other);
        result
    }
}

impl<'a, P: Fp320Parameters> Mul<&'a Fp320<P>> for Fp320<P> {
    type Output = Self;

    #[inline]
    fn mul(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mul_assign(other);
        result
    }
}

impl<'a, P: Fp320Parameters> Div<&'a Fp320<P>> for Fp320<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp320Parameters> AddAssign<&'a Self> for Fp320<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        // This cannot exceed the backing capacity.
        self.0.add_nocarry(&other.0);
        // However, it may need to be reduced
        self.reduce();
    }
}

impl<'a, P: Fp320Parameters> SubAssign<&'a Self> for Fp320<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        // If `other` is larger than `self`, add the modulus to self first.
        if other.0 > self.0 {
            self.0.add_nocarry(&P::MODULUS);
        }

        self.0.sub_noborrow(&other.0);
    }
}

impl<'a, P: Fp320Parameters> MulAssign<&'a Self> for Fp320<P> {
    #[inline]
    fn mul_assign(&mut self, other: &Self) {
        let mut carry = 0;
        let r0 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[0], &mut carry);
        let r1 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[1], &mut carry);
        let r2 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[2], &mut carry);
        let r3 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[3], &mut carry);
        let r4 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[4], &mut carry);
        let r5 = carry;
        let mut carry = 0;
        let r1 = fa::mac_with_carry(r1, (self.0).0[1], (other.0).0[0], &mut carry);
        let r2 = fa::mac_with_carry(r2, (self.0).0[1], (other.0).0[1], &mut carry);
        let r3 = fa::mac_with_carry(r3, (self.0).0[1], (other.0).0[2], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1], (other.0).0[3], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1], (other.0).0[4], &mut carry);
        let r6 = carry;
        let mut carry = 0;
        let r2 = fa::mac_with_carry(r2, (self.0).0[2], (other.0).0[0], &mut carry);
        let r3 = fa::mac_with_carry(r3, (self.0).0[2], (other.0).0[1], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[2], (other.0).0[2], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[2], (other.0).0[3], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2], (other.0).0[4], &mut carry);
        let r7 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[3], (other.0).0[0], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[3], (other.0).0[1], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[3], (other.0).0[2], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[3], (other.0).0[3], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[3], (other.0).0[4], &mut carry);
        let r8 = carry;
        let mut carry = 0;
        let r4 = fa::mac_with_carry(r4, (self.0).0[4], (other.0).0[0], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[4], (other.0).0[1], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[4], (other.0).0[2], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[4], (other.0).0[3], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[4], (other.0).0[4], &mut carry);
        let r9 = carry;
        self.mont_reduce(r0, r1, r2, r3, r4, r5, r6, r7, r8, r9);
    }
}

impl<'a, P: Fp320Parameters> DivAssign<&'a Self> for Fp320<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}
