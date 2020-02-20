use crate::curves::{Field, FpParameters, LegendreSymbol, PrimeField, SquareRootField};
use snarkos_utilities::{
    biginteger::{arithmetic as fa, BigInteger as _BigInteger, BigInteger768 as BigInteger},
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

pub trait Fp768Parameters: FpParameters<BigInt = BigInteger> {}

#[derive(Derivative)]
#[derivative(
    Default(bound = "P: Fp768Parameters"),
    Hash(bound = "P: Fp768Parameters"),
    Clone(bound = "P: Fp768Parameters"),
    Copy(bound = "P: Fp768Parameters"),
    Debug(bound = "P: Fp768Parameters"),
    PartialEq(bound = "P: Fp768Parameters"),
    Eq(bound = "P: Fp768Parameters")
)]
pub struct Fp768<P: Fp768Parameters>(
    pub BigInteger,
    #[derivative(Debug = "ignore")]
    #[doc(hidden)]
    pub PhantomData<P>,
);

impl<P: Fp768Parameters> Fp768<P> {
    #[inline]
    pub fn new(element: BigInteger) -> Self {
        Fp768::<P>(element, PhantomData)
    }

    #[inline]
    pub(crate) fn is_valid(&self) -> bool {
        self.0 < P::MODULUS
    }

    #[inline]
    fn reduce(&mut self) {
        if !self.is_valid() {
            self.0.sub_noborrow(&P::MODULUS);
        }
    }

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
        mut r10: u64,
        mut r11: u64,
        mut r12: u64,
        mut r13: u64,
        mut r14: u64,
        mut r15: u64,
        mut r16: u64,
        mut r17: u64,
        mut r18: u64,
        mut r19: u64,
        mut r20: u64,
        mut r21: u64,
        mut r22: u64,
        mut r23: u64,
    ) {
        let k = r0.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r0, k, P::MODULUS.0[0], &mut carry);
        r1 = fa::mac_with_carry(r1, k, P::MODULUS.0[1], &mut carry);
        r2 = fa::mac_with_carry(r2, k, P::MODULUS.0[2], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[3], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[4], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[5], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[6], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[7], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[8], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[9], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[10], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[11], &mut carry);
        r12 = fa::adc(r12, 0, &mut carry);
        let carry2 = carry;
        let k = r1.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r1, k, P::MODULUS.0[0], &mut carry);
        r2 = fa::mac_with_carry(r2, k, P::MODULUS.0[1], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[2], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[3], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[4], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[5], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[6], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[7], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[8], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[9], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[10], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[11], &mut carry);
        r13 = fa::adc(r13, carry2, &mut carry);
        let carry2 = carry;
        let k = r2.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r2, k, P::MODULUS.0[0], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[1], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[2], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[3], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[4], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[5], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[6], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[7], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[8], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[9], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[10], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[11], &mut carry);
        r14 = fa::adc(r14, carry2, &mut carry);
        let carry2 = carry;
        let k = r3.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r3, k, P::MODULUS.0[0], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[1], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[2], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[3], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[4], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[5], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[6], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[7], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[8], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[9], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[10], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[11], &mut carry);
        r15 = fa::adc(r15, carry2, &mut carry);
        let carry2 = carry;
        let k = r4.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r4, k, P::MODULUS.0[0], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[1], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[2], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[3], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[4], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[5], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[6], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[7], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[8], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[9], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[10], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[11], &mut carry);
        r16 = fa::adc(r16, carry2, &mut carry);
        let carry2 = carry;
        let k = r5.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r5, k, P::MODULUS.0[0], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[1], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[2], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[3], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[4], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[5], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[6], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[7], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[8], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[9], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[10], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[11], &mut carry);
        r17 = fa::adc(r17, carry2, &mut carry);
        let carry2 = carry;
        let k = r6.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r6, k, P::MODULUS.0[0], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[1], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[2], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[3], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[4], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[5], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[6], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[7], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[8], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[9], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[10], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[11], &mut carry);
        r18 = fa::adc(r18, carry2, &mut carry);
        let carry2 = carry;
        let k = r7.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r7, k, P::MODULUS.0[0], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[1], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[2], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[3], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[4], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[5], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[6], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[7], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[8], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[9], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[10], &mut carry);
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[11], &mut carry);
        r19 = fa::adc(r19, carry2, &mut carry);
        let carry2 = carry;
        let k = r8.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r8, k, P::MODULUS.0[0], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[1], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[2], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[3], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[4], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[5], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[6], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[7], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[8], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[9], &mut carry);
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[10], &mut carry);
        r19 = fa::mac_with_carry(r19, k, P::MODULUS.0[11], &mut carry);
        r20 = fa::adc(r20, carry2, &mut carry);
        let carry2 = carry;
        let k = r9.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r9, k, P::MODULUS.0[0], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[1], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[2], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[3], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[4], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[5], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[6], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[7], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[8], &mut carry);
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[9], &mut carry);
        r19 = fa::mac_with_carry(r19, k, P::MODULUS.0[10], &mut carry);
        r20 = fa::mac_with_carry(r20, k, P::MODULUS.0[11], &mut carry);
        r21 = fa::adc(r21, carry2, &mut carry);
        let carry2 = carry;
        let k = r10.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r10, k, P::MODULUS.0[0], &mut carry);
        r11 = fa::mac_with_carry(r11, k, P::MODULUS.0[1], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[2], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[3], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[4], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[5], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[6], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[7], &mut carry);
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[8], &mut carry);
        r19 = fa::mac_with_carry(r19, k, P::MODULUS.0[9], &mut carry);
        r20 = fa::mac_with_carry(r20, k, P::MODULUS.0[10], &mut carry);
        r21 = fa::mac_with_carry(r21, k, P::MODULUS.0[11], &mut carry);
        r22 = fa::adc(r22, carry2, &mut carry);
        let carry2 = carry;
        let k = r11.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r11, k, P::MODULUS.0[0], &mut carry);
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[1], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[2], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[3], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[4], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[5], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[6], &mut carry);
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[7], &mut carry);
        r19 = fa::mac_with_carry(r19, k, P::MODULUS.0[8], &mut carry);
        r20 = fa::mac_with_carry(r20, k, P::MODULUS.0[9], &mut carry);
        r21 = fa::mac_with_carry(r21, k, P::MODULUS.0[10], &mut carry);
        r22 = fa::mac_with_carry(r22, k, P::MODULUS.0[11], &mut carry);
        r23 = fa::adc(r23, carry2, &mut carry);
        (self.0).0[0] = r12;
        (self.0).0[1] = r13;
        (self.0).0[2] = r14;
        (self.0).0[3] = r15;
        (self.0).0[4] = r16;
        (self.0).0[5] = r17;
        (self.0).0[6] = r18;
        (self.0).0[7] = r19;
        (self.0).0[8] = r20;
        (self.0).0[9] = r21;
        (self.0).0[10] = r22;
        (self.0).0[11] = r23;
        self.reduce();
    }
}

impl<P: Fp768Parameters> Field for Fp768<P> {
    #[inline]
    fn zero() -> Self {
        Fp768::<P>(BigInteger::from(0), PhantomData)
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
        Fp768::<P>(P::R, PhantomData)
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
        let r5 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[5], &mut carry);
        let r6 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[6], &mut carry);
        let r7 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[7], &mut carry);
        let r8 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[8], &mut carry);
        let r9 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[9], &mut carry);
        let r10 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[10], &mut carry);
        let r11 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[11], &mut carry);
        let r12 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[1], (self.0).0[2], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1], (self.0).0[3], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1], (self.0).0[4], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[1], (self.0).0[5], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[1], (self.0).0[6], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[1], (self.0).0[7], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[1], (self.0).0[8], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[1], (self.0).0[9], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[1], (self.0).0[10], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[1], (self.0).0[11], &mut carry);
        let r13 = carry;
        let mut carry = 0;
        let r5 = fa::mac_with_carry(r5, (self.0).0[2], (self.0).0[3], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2], (self.0).0[4], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[2], (self.0).0[5], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[2], (self.0).0[6], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[2], (self.0).0[7], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[2], (self.0).0[8], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[2], (self.0).0[9], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[2], (self.0).0[10], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[2], (self.0).0[11], &mut carry);
        let r14 = carry;
        let mut carry = 0;
        let r7 = fa::mac_with_carry(r7, (self.0).0[3], (self.0).0[4], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[3], (self.0).0[5], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[3], (self.0).0[6], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[3], (self.0).0[7], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[3], (self.0).0[8], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[3], (self.0).0[9], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[3], (self.0).0[10], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[3], (self.0).0[11], &mut carry);
        let r15 = carry;
        let mut carry = 0;
        let r9 = fa::mac_with_carry(r9, (self.0).0[4], (self.0).0[5], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[4], (self.0).0[6], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[4], (self.0).0[7], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[4], (self.0).0[8], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[4], (self.0).0[9], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[4], (self.0).0[10], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[4], (self.0).0[11], &mut carry);
        let r16 = carry;
        let mut carry = 0;
        let r11 = fa::mac_with_carry(r11, (self.0).0[5], (self.0).0[6], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[5], (self.0).0[7], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[5], (self.0).0[8], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[5], (self.0).0[9], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[5], (self.0).0[10], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[5], (self.0).0[11], &mut carry);
        let r17 = carry;
        let mut carry = 0;
        let r13 = fa::mac_with_carry(r13, (self.0).0[6], (self.0).0[7], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[6], (self.0).0[8], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[6], (self.0).0[9], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[6], (self.0).0[10], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[6], (self.0).0[11], &mut carry);
        let r18 = carry;
        let mut carry = 0;
        let r15 = fa::mac_with_carry(r15, (self.0).0[7], (self.0).0[8], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[7], (self.0).0[9], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[7], (self.0).0[10], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[7], (self.0).0[11], &mut carry);
        let r19 = carry;
        let mut carry = 0;
        let r17 = fa::mac_with_carry(r17, (self.0).0[8], (self.0).0[9], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[8], (self.0).0[10], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[8], (self.0).0[11], &mut carry);
        let r20 = carry;
        let mut carry = 0;
        let r19 = fa::mac_with_carry(r19, (self.0).0[9], (self.0).0[10], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[9], (self.0).0[11], &mut carry);
        let r21 = carry;
        let mut carry = 0;
        let r21 = fa::mac_with_carry(r21, (self.0).0[10], (self.0).0[11], &mut carry);
        let r22 = carry;

        let tmp0 = r1 >> 63;
        let r1 = r1 << 1;
        let tmp1 = r2 >> 63;
        let r2 = r2 << 1;
        let r2 = r2 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r3 >> 63;
        let r3 = r3 << 1;
        let r3 = r3 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r4 >> 63;
        let r4 = r4 << 1;
        let r4 = r4 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r5 >> 63;
        let r5 = r5 << 1;
        let r5 = r5 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r6 >> 63;
        let r6 = r6 << 1;
        let r6 = r6 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r7 >> 63;
        let r7 = r7 << 1;
        let r7 = r7 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r8 >> 63;
        let r8 = r8 << 1;
        let r8 = r8 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r9 >> 63;
        let r9 = r9 << 1;
        let r9 = r9 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r10 >> 63;
        let r10 = r10 << 1;
        let r10 = r10 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r11 >> 63;
        let r11 = r11 << 1;
        let r11 = r11 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r12 >> 63;
        let r12 = r12 << 1;
        let r12 = r12 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r13 >> 63;
        let r13 = r13 << 1;
        let r13 = r13 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r14 >> 63;
        let r14 = r14 << 1;
        let r14 = r14 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r15 >> 63;
        let r15 = r15 << 1;
        let r15 = r15 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r16 >> 63;
        let r16 = r16 << 1;
        let r16 = r16 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r17 >> 63;
        let r17 = r17 << 1;
        let r17 = r17 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r18 >> 63;
        let r18 = r18 << 1;
        let r18 = r18 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r19 >> 63;
        let r19 = r19 << 1;
        let r19 = r19 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r20 >> 63;
        let r20 = r20 << 1;
        let r20 = r20 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r21 >> 63;
        let r21 = r21 << 1;
        let r21 = r21 | tmp0;
        let tmp0 = tmp1;
        let tmp1 = r22 >> 63;
        let r22 = r22 << 1;
        let r22 = r22 | tmp0;
        let tmp0 = tmp1;
        let r23 = tmp0;

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
        let r10 = fa::mac_with_carry(r10, (self.0).0[5], (self.0).0[5], &mut carry);
        let r11 = fa::adc(r11, 0, &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[6], (self.0).0[6], &mut carry);
        let r13 = fa::adc(r13, 0, &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[7], (self.0).0[7], &mut carry);
        let r15 = fa::adc(r15, 0, &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[8], (self.0).0[8], &mut carry);
        let r17 = fa::adc(r17, 0, &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[9], (self.0).0[9], &mut carry);
        let r19 = fa::adc(r19, 0, &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[10], (self.0).0[10], &mut carry);
        let r21 = fa::adc(r21, 0, &mut carry);
        let r22 = fa::mac_with_carry(r22, (self.0).0[11], (self.0).0[11], &mut carry);
        let r23 = fa::adc(r23, 0, &mut carry);

        self.mont_reduce(
            r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22,
            r23,
        );
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
            let mut b = Fp768::<P>(P::R2, PhantomData); // Avoids unnecessary reduction step.
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

impl<P: Fp768Parameters> PrimeField for Fp768<P> {
    type BigInt = BigInteger;
    type Params = P;

    #[inline]
    fn from_repr(r: BigInteger) -> Self {
        let mut r = Fp768(r, PhantomData);
        if r.is_valid() {
            r.mul_assign(&Fp768(P::R2, PhantomData));
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
            (self.0).0[5],
            (self.0).0[6],
            (self.0).0[7],
            (self.0).0[8],
            (self.0).0[9],
            (self.0).0[10],
            (self.0).0[11],
            0,
            0,
            0,
            0,
            0,
            0,
            0,
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
        let r = Fp768(r, PhantomData);
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
            result.0.as_mut()[11] &= 0xffffffffffffffff >> P::REPR_SHAVE_BITS;
            if result.is_valid() { Some(result) } else { None }
        } else {
            None
        }
    }

    #[inline]
    fn multiplicative_generator() -> Self {
        Fp768::<P>(P::GENERATOR, PhantomData)
    }

    #[inline]
    fn root_of_unity() -> Self {
        Fp768::<P>(P::ROOT_OF_UNITY, PhantomData)
    }

    #[inline]
    fn size_in_bits() -> usize {
        P::MODULUS_BITS as usize
    }

    #[inline]
    fn trace() -> BigInteger {
        P::T
    }

    #[inline]
    fn trace_minus_one_div_two() -> BigInteger {
        P::T_MINUS_ONE_DIV_TWO
    }

    #[inline]
    fn modulus_minus_one_div_two() -> BigInteger {
        P::MODULUS_MINUS_ONE_DIV_TWO
    }
}

impl<P: Fp768Parameters> SquareRootField for Fp768<P> {
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

impl<P: Fp768Parameters> Ord for Fp768<P> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.into_repr().cmp(&other.into_repr())
    }
}

impl<P: Fp768Parameters> PartialOrd for Fp768<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl_prime_field_from_int!(Fp768, u128, Fp768Parameters);
impl_prime_field_from_int!(Fp768, u64, Fp768Parameters);
impl_prime_field_from_int!(Fp768, u32, Fp768Parameters);
impl_prime_field_from_int!(Fp768, u16, Fp768Parameters);
impl_prime_field_from_int!(Fp768, u8, Fp768Parameters);

impl_prime_field_standard_sample!(Fp768, Fp768Parameters);

impl<P: Fp768Parameters> ToBytes for Fp768<P> {
    #[inline]
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.into_repr().write(writer)
    }
}

impl<P: Fp768Parameters> FromBytes for Fp768<P> {
    #[inline]
    fn read<R: Read>(reader: R) -> IoResult<Self> {
        BigInteger::read(reader).map(Fp768::from_repr)
    }
}

impl<P: Fp768Parameters> FromStr for Fp768<P> {
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

impl<P: Fp768Parameters> Display for Fp768<P> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Fp768({})", self.into_repr())
    }
}

impl<P: Fp768Parameters> Neg for Fp768<P> {
    type Output = Self;

    #[inline]
    #[must_use]
    fn neg(self) -> Self {
        if !self.is_zero() {
            let mut tmp = P::MODULUS.clone();
            tmp.sub_noborrow(&self.0);
            Fp768::<P>(tmp, PhantomData)
        } else {
            self
        }
    }
}

impl<'a, P: Fp768Parameters> Add<&'a Fp768<P>> for Fp768<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.add_assign(other);
        result
    }
}

impl<'a, P: Fp768Parameters> Sub<&'a Fp768<P>> for Fp768<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.sub_assign(other);
        result
    }
}

impl<'a, P: Fp768Parameters> Mul<&'a Fp768<P>> for Fp768<P> {
    type Output = Self;

    #[inline]
    fn mul(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mul_assign(other);
        result
    }
}

impl<'a, P: Fp768Parameters> Div<&'a Fp768<P>> for Fp768<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp768Parameters> AddAssign<&'a Self> for Fp768<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        // This cannot exceed the backing capacity.
        self.0.add_nocarry(&other.0);
        // However, it may need to be reduced
        self.reduce();
    }
}

impl<'a, P: Fp768Parameters> SubAssign<&'a Self> for Fp768<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        // If `other` is larger than `self`, add the modulus to self first.
        if other.0 > self.0 {
            self.0.add_nocarry(&P::MODULUS);
        }

        self.0.sub_noborrow(&other.0);
    }
}

impl<'a, P: Fp768Parameters> MulAssign<&'a Self> for Fp768<P> {
    #[inline]
    fn mul_assign(&mut self, other: &Self) {
        let mut carry = 0;
        let r0 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[0], &mut carry);
        let r1 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[1], &mut carry);
        let r2 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[2], &mut carry);
        let r3 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[3], &mut carry);
        let r4 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[4], &mut carry);
        let r5 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[5], &mut carry);
        let r6 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[6], &mut carry);
        let r7 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[7], &mut carry);
        let r8 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[8], &mut carry);
        let r9 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[9], &mut carry);
        let r10 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[10], &mut carry);
        let r11 = fa::mac_with_carry(0, (self.0).0[0], (other.0).0[11], &mut carry);
        let r12 = carry;
        let mut carry = 0;
        let r1 = fa::mac_with_carry(r1, (self.0).0[1], (other.0).0[0], &mut carry);
        let r2 = fa::mac_with_carry(r2, (self.0).0[1], (other.0).0[1], &mut carry);
        let r3 = fa::mac_with_carry(r3, (self.0).0[1], (other.0).0[2], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1], (other.0).0[3], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1], (other.0).0[4], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[1], (other.0).0[5], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[1], (other.0).0[6], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[1], (other.0).0[7], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[1], (other.0).0[8], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[1], (other.0).0[9], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[1], (other.0).0[10], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[1], (other.0).0[11], &mut carry);
        let r13 = carry;
        let mut carry = 0;
        let r2 = fa::mac_with_carry(r2, (self.0).0[2], (other.0).0[0], &mut carry);
        let r3 = fa::mac_with_carry(r3, (self.0).0[2], (other.0).0[1], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[2], (other.0).0[2], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[2], (other.0).0[3], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2], (other.0).0[4], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[2], (other.0).0[5], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[2], (other.0).0[6], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[2], (other.0).0[7], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[2], (other.0).0[8], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[2], (other.0).0[9], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[2], (other.0).0[10], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[2], (other.0).0[11], &mut carry);
        let r14 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[3], (other.0).0[0], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[3], (other.0).0[1], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[3], (other.0).0[2], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[3], (other.0).0[3], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[3], (other.0).0[4], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[3], (other.0).0[5], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[3], (other.0).0[6], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[3], (other.0).0[7], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[3], (other.0).0[8], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[3], (other.0).0[9], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[3], (other.0).0[10], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[3], (other.0).0[11], &mut carry);
        let r15 = carry;
        let mut carry = 0;
        let r4 = fa::mac_with_carry(r4, (self.0).0[4], (other.0).0[0], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[4], (other.0).0[1], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[4], (other.0).0[2], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[4], (other.0).0[3], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[4], (other.0).0[4], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[4], (other.0).0[5], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[4], (other.0).0[6], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[4], (other.0).0[7], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[4], (other.0).0[8], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[4], (other.0).0[9], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[4], (other.0).0[10], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[4], (other.0).0[11], &mut carry);
        let r16 = carry;
        let mut carry = 0;
        let r5 = fa::mac_with_carry(r5, (self.0).0[5], (other.0).0[0], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[5], (other.0).0[1], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[5], (other.0).0[2], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[5], (other.0).0[3], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[5], (other.0).0[4], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[5], (other.0).0[5], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[5], (other.0).0[6], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[5], (other.0).0[7], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[5], (other.0).0[8], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[5], (other.0).0[9], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[5], (other.0).0[10], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[5], (other.0).0[11], &mut carry);
        let r17 = carry;
        let mut carry = 0;
        let r6 = fa::mac_with_carry(r6, (self.0).0[6], (other.0).0[0], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[6], (other.0).0[1], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[6], (other.0).0[2], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[6], (other.0).0[3], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[6], (other.0).0[4], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[6], (other.0).0[5], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[6], (other.0).0[6], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[6], (other.0).0[7], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[6], (other.0).0[8], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[6], (other.0).0[9], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[6], (other.0).0[10], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[6], (other.0).0[11], &mut carry);
        let r18 = carry;
        let mut carry = 0;
        let r7 = fa::mac_with_carry(r7, (self.0).0[7], (other.0).0[0], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[7], (other.0).0[1], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[7], (other.0).0[2], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[7], (other.0).0[3], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[7], (other.0).0[4], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[7], (other.0).0[5], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[7], (other.0).0[6], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[7], (other.0).0[7], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[7], (other.0).0[8], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[7], (other.0).0[9], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[7], (other.0).0[10], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[7], (other.0).0[11], &mut carry);
        let r19 = carry;
        let mut carry = 0;
        let r8 = fa::mac_with_carry(r8, (self.0).0[8], (other.0).0[0], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[8], (other.0).0[1], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[8], (other.0).0[2], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[8], (other.0).0[3], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[8], (other.0).0[4], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[8], (other.0).0[5], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[8], (other.0).0[6], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[8], (other.0).0[7], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[8], (other.0).0[8], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[8], (other.0).0[9], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[8], (other.0).0[10], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[8], (other.0).0[11], &mut carry);
        let r20 = carry;
        let mut carry = 0;
        let r9 = fa::mac_with_carry(r9, (self.0).0[9], (other.0).0[0], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[9], (other.0).0[1], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[9], (other.0).0[2], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[9], (other.0).0[3], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[9], (other.0).0[4], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[9], (other.0).0[5], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[9], (other.0).0[6], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[9], (other.0).0[7], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[9], (other.0).0[8], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[9], (other.0).0[9], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[9], (other.0).0[10], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[9], (other.0).0[11], &mut carry);
        let r21 = carry;
        let mut carry = 0;
        let r10 = fa::mac_with_carry(r10, (self.0).0[10], (other.0).0[0], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[10], (other.0).0[1], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[10], (other.0).0[2], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[10], (other.0).0[3], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[10], (other.0).0[4], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[10], (other.0).0[5], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[10], (other.0).0[6], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[10], (other.0).0[7], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[10], (other.0).0[8], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[10], (other.0).0[9], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[10], (other.0).0[10], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[10], (other.0).0[11], &mut carry);
        let r22 = carry;
        let mut carry = 0;
        let r11 = fa::mac_with_carry(r11, (self.0).0[11], (other.0).0[0], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[11], (other.0).0[1], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[11], (other.0).0[2], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[11], (other.0).0[3], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[11], (other.0).0[4], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[11], (other.0).0[5], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[11], (other.0).0[6], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[11], (other.0).0[7], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[11], (other.0).0[8], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[11], (other.0).0[9], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[11], (other.0).0[10], &mut carry);
        let r22 = fa::mac_with_carry(r22, (self.0).0[11], (other.0).0[11], &mut carry);
        let r23 = carry;
        self.mont_reduce(
            r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22,
            r23,
        );
    }
}

impl<'a, P: Fp768Parameters> DivAssign<&'a Self> for Fp768<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}
