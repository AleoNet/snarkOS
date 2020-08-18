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

use crate::curves::{Field, FpParameters, LegendreSymbol, One, PrimeField, SquareRootField, Zero};
use snarkos_errors::curves::FieldError;
use snarkos_utilities::{
    biginteger::{arithmetic as fa, BigInteger as _BigInteger, BigInteger832 as BigInteger},
    bytes::{FromBytes, ToBytes},
    serialize::CanonicalDeserialize,
};

use std::{
    cmp::{Ord, Ordering, PartialOrd},
    fmt::{Display, Formatter, Result as FmtResult},
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

pub trait Fp832Parameters: FpParameters<BigInteger = BigInteger> {}

#[derive(Derivative)]
#[derivative(
    Default(bound = "P: Fp832Parameters"),
    Hash(bound = "P: Fp832Parameters"),
    Clone(bound = "P: Fp832Parameters"),
    Copy(bound = "P: Fp832Parameters"),
    Debug(bound = "P: Fp832Parameters"),
    PartialEq(bound = "P: Fp832Parameters"),
    Eq(bound = "P: Fp832Parameters")
)]
pub struct Fp832<P: Fp832Parameters>(
    pub BigInteger,
    #[derivative(Debug = "ignore")]
    #[doc(hidden)]
    pub PhantomData<P>,
);

impl<P: Fp832Parameters> Fp832<P> {
    #[inline]
    pub fn new(element: BigInteger) -> Self {
        Fp832::<P>(element, PhantomData)
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
        mut r24: u64,
        mut r25: u64,
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
        r12 = fa::mac_with_carry(r12, k, P::MODULUS.0[12], &mut carry);
        r13 = fa::adc(r13, 0, &mut carry);
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
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[12], &mut carry);
        r14 = fa::adc(r14, carry2, &mut carry);
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
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[12], &mut carry);
        r15 = fa::adc(r15, carry2, &mut carry);
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
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[12], &mut carry);
        r16 = fa::adc(r16, carry2, &mut carry);
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
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[12], &mut carry);
        r17 = fa::adc(r17, carry2, &mut carry);
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
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[12], &mut carry);
        r18 = fa::adc(r18, carry2, &mut carry);
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
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[12], &mut carry);
        r19 = fa::adc(r19, carry2, &mut carry);
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
        r19 = fa::mac_with_carry(r19, k, P::MODULUS.0[12], &mut carry);
        r20 = fa::adc(r20, carry2, &mut carry);
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
        r20 = fa::mac_with_carry(r20, k, P::MODULUS.0[12], &mut carry);
        r21 = fa::adc(r21, carry2, &mut carry);
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
        r21 = fa::mac_with_carry(r21, k, P::MODULUS.0[12], &mut carry);
        r22 = fa::adc(r22, carry2, &mut carry);
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
        r22 = fa::mac_with_carry(r22, k, P::MODULUS.0[12], &mut carry);
        r23 = fa::adc(r23, carry2, &mut carry);
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
        r23 = fa::mac_with_carry(r23, k, P::MODULUS.0[12], &mut carry);
        r24 = fa::adc(r24, carry2, &mut carry);
        let carry2 = carry;
        let k = r12.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r12, k, P::MODULUS.0[0], &mut carry);
        r13 = fa::mac_with_carry(r13, k, P::MODULUS.0[1], &mut carry);
        r14 = fa::mac_with_carry(r14, k, P::MODULUS.0[2], &mut carry);
        r15 = fa::mac_with_carry(r15, k, P::MODULUS.0[3], &mut carry);
        r16 = fa::mac_with_carry(r16, k, P::MODULUS.0[4], &mut carry);
        r17 = fa::mac_with_carry(r17, k, P::MODULUS.0[5], &mut carry);
        r18 = fa::mac_with_carry(r18, k, P::MODULUS.0[6], &mut carry);
        r19 = fa::mac_with_carry(r19, k, P::MODULUS.0[7], &mut carry);
        r20 = fa::mac_with_carry(r20, k, P::MODULUS.0[8], &mut carry);
        r21 = fa::mac_with_carry(r21, k, P::MODULUS.0[9], &mut carry);
        r22 = fa::mac_with_carry(r22, k, P::MODULUS.0[10], &mut carry);
        r23 = fa::mac_with_carry(r23, k, P::MODULUS.0[11], &mut carry);
        r24 = fa::mac_with_carry(r24, k, P::MODULUS.0[12], &mut carry);
        r25 = fa::adc(r25, carry2, &mut carry);
        (self.0).0[0] = r13;
        (self.0).0[1] = r14;
        (self.0).0[2] = r15;
        (self.0).0[3] = r16;
        (self.0).0[4] = r17;
        (self.0).0[5] = r18;
        (self.0).0[6] = r19;
        (self.0).0[7] = r20;
        (self.0).0[8] = r21;
        (self.0).0[9] = r22;
        (self.0).0[10] = r23;
        (self.0).0[11] = r24;
        (self.0).0[12] = r25;
        self.reduce();
    }
}

impl<P: Fp832Parameters> Zero for Fp832<P> {
    #[inline]
    fn zero() -> Self {
        Fp832::<P>(BigInteger::from(0), PhantomData)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<P: Fp832Parameters> One for Fp832<P> {
    #[inline]
    fn one() -> Self {
        Fp832::<P>(P::R, PhantomData)
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.0 == P::R
    }
}

impl<P: Fp832Parameters> Field for Fp832<P> {
    // 832/64 = 13 limbs.
    impl_field_from_random_bytes_with_flags!(13);

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
    fn characteristic<'a>() -> &'a [u64] {
        P::MODULUS.as_ref()
    }

    #[inline]
    fn square(&self) -> Self {
        let mut temp = self.clone();
        temp.square_in_place();
        temp
    }

    fn square_in_place(&mut self) -> &mut Self {
        let mut carry = 0;
        let r1 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[1usize], &mut carry);
        let r2 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[2usize], &mut carry);
        let r3 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[3usize], &mut carry);
        let r4 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[4usize], &mut carry);
        let r5 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[5usize], &mut carry);
        let r6 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[6usize], &mut carry);
        let r7 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[7usize], &mut carry);
        let r8 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[8usize], &mut carry);
        let r9 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[9usize], &mut carry);
        let r10 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[10usize], &mut carry);
        let r11 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[11usize], &mut carry);
        let r12 = fa::mac_with_carry(0, (self.0).0[0usize], (self.0).0[12usize], &mut carry);
        let r13 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[1usize], (self.0).0[2usize], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1usize], (self.0).0[3usize], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1usize], (self.0).0[4usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[1usize], (self.0).0[5usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[1usize], (self.0).0[6usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[1usize], (self.0).0[7usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[1usize], (self.0).0[8usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[1usize], (self.0).0[9usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[1usize], (self.0).0[10usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[1usize], (self.0).0[11usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[1usize], (self.0).0[12usize], &mut carry);
        let r14 = carry;
        let mut carry = 0;
        let r5 = fa::mac_with_carry(r5, (self.0).0[2usize], (self.0).0[3usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2usize], (self.0).0[4usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[2usize], (self.0).0[5usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[2usize], (self.0).0[6usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[2usize], (self.0).0[7usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[2usize], (self.0).0[8usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[2usize], (self.0).0[9usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[2usize], (self.0).0[10usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[2usize], (self.0).0[11usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[2usize], (self.0).0[12usize], &mut carry);
        let r15 = carry;
        let mut carry = 0;
        let r7 = fa::mac_with_carry(r7, (self.0).0[3usize], (self.0).0[4usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[3usize], (self.0).0[5usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[3usize], (self.0).0[6usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[3usize], (self.0).0[7usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[3usize], (self.0).0[8usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[3usize], (self.0).0[9usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[3usize], (self.0).0[10usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[3usize], (self.0).0[11usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[3usize], (self.0).0[12usize], &mut carry);
        let r16 = carry;
        let mut carry = 0;
        let r9 = fa::mac_with_carry(r9, (self.0).0[4usize], (self.0).0[5usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[4usize], (self.0).0[6usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[4usize], (self.0).0[7usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[4usize], (self.0).0[8usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[4usize], (self.0).0[9usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[4usize], (self.0).0[10usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[4usize], (self.0).0[11usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[4usize], (self.0).0[12usize], &mut carry);
        let r17 = carry;
        let mut carry = 0;
        let r11 = fa::mac_with_carry(r11, (self.0).0[5usize], (self.0).0[6usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[5usize], (self.0).0[7usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[5usize], (self.0).0[8usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[5usize], (self.0).0[9usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[5usize], (self.0).0[10usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[5usize], (self.0).0[11usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[5usize], (self.0).0[12usize], &mut carry);
        let r18 = carry;
        let mut carry = 0;
        let r13 = fa::mac_with_carry(r13, (self.0).0[6usize], (self.0).0[7usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[6usize], (self.0).0[8usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[6usize], (self.0).0[9usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[6usize], (self.0).0[10usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[6usize], (self.0).0[11usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[6usize], (self.0).0[12usize], &mut carry);
        let r19 = carry;
        let mut carry = 0;
        let r15 = fa::mac_with_carry(r15, (self.0).0[7usize], (self.0).0[8usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[7usize], (self.0).0[9usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[7usize], (self.0).0[10usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[7usize], (self.0).0[11usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[7usize], (self.0).0[12usize], &mut carry);
        let r20 = carry;
        let mut carry = 0;
        let r17 = fa::mac_with_carry(r17, (self.0).0[8usize], (self.0).0[9usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[8usize], (self.0).0[10usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[8usize], (self.0).0[11usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[8usize], (self.0).0[12usize], &mut carry);
        let r21 = carry;
        let mut carry = 0;
        let r19 = fa::mac_with_carry(r19, (self.0).0[9usize], (self.0).0[10usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[9usize], (self.0).0[11usize], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[9usize], (self.0).0[12usize], &mut carry);
        let r22 = carry;
        let mut carry = 0;
        let r21 = fa::mac_with_carry(r21, (self.0).0[10usize], (self.0).0[11usize], &mut carry);
        let r22 = fa::mac_with_carry(r22, (self.0).0[10usize], (self.0).0[12usize], &mut carry);
        let r23 = carry;
        let mut carry = 0;
        let r23 = fa::mac_with_carry(r23, (self.0).0[11usize], (self.0).0[12usize], &mut carry);
        let r24 = carry;
        let r25 = r24 >> 63;
        let r24 = (r24 << 1) | (r23 >> 63);
        let r23 = (r23 << 1) | (r22 >> 63);
        let r22 = (r22 << 1) | (r21 >> 63);
        let r21 = (r21 << 1) | (r20 >> 63);
        let r20 = (r20 << 1) | (r19 >> 63);
        let r19 = (r19 << 1) | (r18 >> 63);
        let r18 = (r18 << 1) | (r17 >> 63);
        let r17 = (r17 << 1) | (r16 >> 63);
        let r16 = (r16 << 1) | (r15 >> 63);
        let r15 = (r15 << 1) | (r14 >> 63);
        let r14 = (r14 << 1) | (r13 >> 63);
        let r13 = (r13 << 1) | (r12 >> 63);
        let r12 = (r12 << 1) | (r11 >> 63);
        let r11 = (r11 << 1) | (r10 >> 63);
        let r10 = (r10 << 1) | (r9 >> 63);
        let r9 = (r9 << 1) | (r8 >> 63);
        let r8 = (r8 << 1) | (r7 >> 63);
        let r7 = (r7 << 1) | (r6 >> 63);
        let r6 = (r6 << 1) | (r5 >> 63);
        let r5 = (r5 << 1) | (r4 >> 63);
        let r4 = (r4 << 1) | (r3 >> 63);
        let r3 = (r3 << 1) | (r2 >> 63);
        let r2 = (r2 << 1) | (r1 >> 63);
        let r1 = r1 << 1;

        let mut r_s = [
            0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22,
            r23, r24, r25,
        ];

        let mut carry = 0;
        for i in 0..13 {
            r_s[2 * i] = fa::mac_with_carry(r_s[2 * i], (self.0).0[i], (self.0).0[i], &mut carry);
            r_s[2 * i + 1] = fa::adc(r_s[2 * i + 1], 0, &mut carry);
        }
        self.mont_reduce(
            r_s[0], r_s[1], r_s[2], r_s[3], r_s[4], r_s[5], r_s[6], r_s[7], r_s[8], r_s[9], r_s[10], r_s[11], r_s[12],
            r_s[13], r_s[14], r_s[15], r_s[16], r_s[17], r_s[18], r_s[19], r_s[20], r_s[21], r_s[22], r_s[23], r_s[24],
            r_s[25],
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
            let mut b = Fp832::<P>(P::R2, PhantomData); // Avoids unnecessary reduction step.
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

impl<P: Fp832Parameters> PrimeField for Fp832<P> {
    type BigInteger = BigInteger;
    type Parameters = P;

    #[inline]
    fn from_repr(r: BigInteger) -> Option<Self> {
        let mut r = Fp832(r, PhantomData);
        if r.is_zero() {
            Some(r)
        } else if r.is_valid() {
            r *= &Fp832(P::R2, PhantomData);
            Some(r)
        } else {
            None
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
            (self.0).0[12],
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
            0,
        );
        r.0
    }

    #[inline]
    fn from_repr_raw(r: BigInteger) -> Self {
        let r = Fp832(r, PhantomData);
        if r.is_valid() { r } else { Self::zero() }
    }

    #[inline]
    fn into_repr_raw(&self) -> BigInteger {
        let r = *self;
        r.0
    }

    #[inline]
    fn multiplicative_generator() -> Self {
        Fp832::<P>(P::GENERATOR, PhantomData)
    }

    #[inline]
    fn root_of_unity() -> Self {
        Fp832::<P>(P::ROOT_OF_UNITY, PhantomData)
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

impl<P: Fp832Parameters> SquareRootField for Fp832<P> {
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

impl<P: Fp832Parameters> Ord for Fp832<P> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.into_repr().cmp(&other.into_repr())
    }
}

impl<P: Fp832Parameters> PartialOrd for Fp832<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl_prime_field_from_int!(Fp832, u128, Fp832Parameters);
impl_prime_field_from_int!(Fp832, u64, Fp832Parameters);
impl_prime_field_from_int!(Fp832, u32, Fp832Parameters);
impl_prime_field_from_int!(Fp832, u16, Fp832Parameters);
impl_prime_field_from_int!(Fp832, u8, Fp832Parameters);

impl_prime_field_standard_sample!(Fp832, Fp832Parameters);

impl<P: Fp832Parameters> ToBytes for Fp832<P> {
    #[inline]
    fn write<W: Write>(&self, writer: W) -> IoResult<()> {
        self.into_repr().write(writer)
    }
}

impl<P: Fp832Parameters> FromBytes for Fp832<P> {
    #[inline]
    fn read<R: Read>(reader: R) -> IoResult<Self> {
        BigInteger::read(reader).and_then(|b| match Self::from_repr(b) {
            Some(f) => Ok(f),
            None => Err(FieldError::InvalidFieldElement.into()),
        })
    }
}

impl<P: Fp832Parameters> FromStr for Fp832<P> {
    type Err = FieldError;

    /// Interpret a string of numbers as a (congruent) prime field element.
    /// Does not accept unnecessary leading zeroes or a blank string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(FieldError::ParsingEmptyString);
        }

        if s == "0" {
            return Ok(Self::zero());
        }

        let mut res = Self::zero();

        let ten = Self::from_repr(<Self as PrimeField>::BigInteger::from(10)).ok_or(FieldError::InvalidFieldElement)?;

        let mut first_digit = true;

        for c in s.chars() {
            match c.to_digit(10) {
                Some(c) => {
                    if first_digit {
                        if c == 0 {
                            return Err(FieldError::InvalidString);
                        }

                        first_digit = false;
                    }

                    res.mul_assign(&ten);
                    res.add_assign(
                        &Self::from_repr(<Self as PrimeField>::BigInteger::from(u64::from(c)))
                            .ok_or(FieldError::InvalidFieldElement)?,
                    );
                }
                None => {
                    return Err(FieldError::ParsingNonDigitCharacter);
                }
            }
        }

        if !res.is_valid() {
            Err(FieldError::InvalidFieldElement)
        } else {
            Ok(res)
        }
    }
}

impl<P: Fp832Parameters> Display for Fp832<P> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Fp832({})", self.into_repr())
    }
}

impl<P: Fp832Parameters> Neg for Fp832<P> {
    type Output = Self;

    #[inline]
    #[must_use]
    fn neg(self) -> Self {
        if !self.is_zero() {
            let mut tmp = P::MODULUS.clone();
            tmp.sub_noborrow(&self.0);
            Fp832::<P>(tmp, PhantomData)
        } else {
            self
        }
    }
}

impl<'a, P: Fp832Parameters> Add<&'a Fp832<P>> for Fp832<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.add_assign(other);
        result
    }
}

impl<'a, P: Fp832Parameters> Sub<&'a Fp832<P>> for Fp832<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.sub_assign(other);
        result
    }
}

impl<'a, P: Fp832Parameters> Mul<&'a Fp832<P>> for Fp832<P> {
    type Output = Self;

    #[inline]
    fn mul(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mul_assign(other);
        result
    }
}

impl<'a, P: Fp832Parameters> Div<&'a Fp832<P>> for Fp832<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self.clone();
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp832Parameters> AddAssign<&'a Self> for Fp832<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        // This cannot exceed the backing capacity.
        self.0.add_nocarry(&other.0);
        // However, it may need to be reduced
        self.reduce();
    }
}

impl<'a, P: Fp832Parameters> SubAssign<&'a Self> for Fp832<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        // If `other` is larger than `self`, add the modulus to self first.
        if other.0 > self.0 {
            self.0.add_nocarry(&P::MODULUS);
        }

        self.0.sub_noborrow(&other.0);
    }
}

impl<'a, P: Fp832Parameters> MulAssign<&'a Self> for Fp832<P> {
    #[inline]
    fn mul_assign(&mut self, other: &Self) {
        let mut carry = 0;
        let r0 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[0usize], &mut carry);
        let r1 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[1usize], &mut carry);
        let r2 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[2usize], &mut carry);
        let r3 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[3usize], &mut carry);
        let r4 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[4usize], &mut carry);
        let r5 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[5usize], &mut carry);
        let r6 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[6usize], &mut carry);
        let r7 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[7usize], &mut carry);
        let r8 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[8usize], &mut carry);
        let r9 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[9usize], &mut carry);
        let r10 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[10usize], &mut carry);
        let r11 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[11usize], &mut carry);
        let r12 = fa::mac_with_carry(0, (self.0).0[0usize], (other.0).0[12usize], &mut carry);
        let r13 = carry;
        let mut carry = 0;
        let r1 = fa::mac_with_carry(r1, (self.0).0[1usize], (other.0).0[0usize], &mut carry);
        let r2 = fa::mac_with_carry(r2, (self.0).0[1usize], (other.0).0[1usize], &mut carry);
        let r3 = fa::mac_with_carry(r3, (self.0).0[1usize], (other.0).0[2usize], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1usize], (other.0).0[3usize], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1usize], (other.0).0[4usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[1usize], (other.0).0[5usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[1usize], (other.0).0[6usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[1usize], (other.0).0[7usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[1usize], (other.0).0[8usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[1usize], (other.0).0[9usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[1usize], (other.0).0[10usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[1usize], (other.0).0[11usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[1usize], (other.0).0[12usize], &mut carry);
        let r14 = carry;
        let mut carry = 0;
        let r2 = fa::mac_with_carry(r2, (self.0).0[2usize], (other.0).0[0usize], &mut carry);
        let r3 = fa::mac_with_carry(r3, (self.0).0[2usize], (other.0).0[1usize], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[2usize], (other.0).0[2usize], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[2usize], (other.0).0[3usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2usize], (other.0).0[4usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[2usize], (other.0).0[5usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[2usize], (other.0).0[6usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[2usize], (other.0).0[7usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[2usize], (other.0).0[8usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[2usize], (other.0).0[9usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[2usize], (other.0).0[10usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[2usize], (other.0).0[11usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[2usize], (other.0).0[12usize], &mut carry);
        let r15 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[3usize], (other.0).0[0usize], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[3usize], (other.0).0[1usize], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[3usize], (other.0).0[2usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[3usize], (other.0).0[3usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[3usize], (other.0).0[4usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[3usize], (other.0).0[5usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[3usize], (other.0).0[6usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[3usize], (other.0).0[7usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[3usize], (other.0).0[8usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[3usize], (other.0).0[9usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[3usize], (other.0).0[10usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[3usize], (other.0).0[11usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[3usize], (other.0).0[12usize], &mut carry);
        let r16 = carry;
        let mut carry = 0;
        let r4 = fa::mac_with_carry(r4, (self.0).0[4usize], (other.0).0[0usize], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[4usize], (other.0).0[1usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[4usize], (other.0).0[2usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[4usize], (other.0).0[3usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[4usize], (other.0).0[4usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[4usize], (other.0).0[5usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[4usize], (other.0).0[6usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[4usize], (other.0).0[7usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[4usize], (other.0).0[8usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[4usize], (other.0).0[9usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[4usize], (other.0).0[10usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[4usize], (other.0).0[11usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[4usize], (other.0).0[12usize], &mut carry);
        let r17 = carry;
        let mut carry = 0;
        let r5 = fa::mac_with_carry(r5, (self.0).0[5usize], (other.0).0[0usize], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[5usize], (other.0).0[1usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[5usize], (other.0).0[2usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[5usize], (other.0).0[3usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[5usize], (other.0).0[4usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[5usize], (other.0).0[5usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[5usize], (other.0).0[6usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[5usize], (other.0).0[7usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[5usize], (other.0).0[8usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[5usize], (other.0).0[9usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[5usize], (other.0).0[10usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[5usize], (other.0).0[11usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[5usize], (other.0).0[12usize], &mut carry);
        let r18 = carry;
        let mut carry = 0;
        let r6 = fa::mac_with_carry(r6, (self.0).0[6usize], (other.0).0[0usize], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[6usize], (other.0).0[1usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[6usize], (other.0).0[2usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[6usize], (other.0).0[3usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[6usize], (other.0).0[4usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[6usize], (other.0).0[5usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[6usize], (other.0).0[6usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[6usize], (other.0).0[7usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[6usize], (other.0).0[8usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[6usize], (other.0).0[9usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[6usize], (other.0).0[10usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[6usize], (other.0).0[11usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[6usize], (other.0).0[12usize], &mut carry);
        let r19 = carry;
        let mut carry = 0;
        let r7 = fa::mac_with_carry(r7, (self.0).0[7usize], (other.0).0[0usize], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[7usize], (other.0).0[1usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[7usize], (other.0).0[2usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[7usize], (other.0).0[3usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[7usize], (other.0).0[4usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[7usize], (other.0).0[5usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[7usize], (other.0).0[6usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[7usize], (other.0).0[7usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[7usize], (other.0).0[8usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[7usize], (other.0).0[9usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[7usize], (other.0).0[10usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[7usize], (other.0).0[11usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[7usize], (other.0).0[12usize], &mut carry);
        let r20 = carry;
        let mut carry = 0;
        let r8 = fa::mac_with_carry(r8, (self.0).0[8usize], (other.0).0[0usize], &mut carry);
        let r9 = fa::mac_with_carry(r9, (self.0).0[8usize], (other.0).0[1usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[8usize], (other.0).0[2usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[8usize], (other.0).0[3usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[8usize], (other.0).0[4usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[8usize], (other.0).0[5usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[8usize], (other.0).0[6usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[8usize], (other.0).0[7usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[8usize], (other.0).0[8usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[8usize], (other.0).0[9usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[8usize], (other.0).0[10usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[8usize], (other.0).0[11usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[8usize], (other.0).0[12usize], &mut carry);
        let r21 = carry;
        let mut carry = 0;
        let r9 = fa::mac_with_carry(r9, (self.0).0[9usize], (other.0).0[0usize], &mut carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[9usize], (other.0).0[1usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[9usize], (other.0).0[2usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[9usize], (other.0).0[3usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[9usize], (other.0).0[4usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[9usize], (other.0).0[5usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[9usize], (other.0).0[6usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[9usize], (other.0).0[7usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[9usize], (other.0).0[8usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[9usize], (other.0).0[9usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[9usize], (other.0).0[10usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[9usize], (other.0).0[11usize], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[9usize], (other.0).0[12usize], &mut carry);
        let r22 = carry;
        let mut carry = 0;
        let r10 = fa::mac_with_carry(r10, (self.0).0[10usize], (other.0).0[0usize], &mut carry);
        let r11 = fa::mac_with_carry(r11, (self.0).0[10usize], (other.0).0[1usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[10usize], (other.0).0[2usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[10usize], (other.0).0[3usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[10usize], (other.0).0[4usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[10usize], (other.0).0[5usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[10usize], (other.0).0[6usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[10usize], (other.0).0[7usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[10usize], (other.0).0[8usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[10usize], (other.0).0[9usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[10usize], (other.0).0[10usize], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[10usize], (other.0).0[11usize], &mut carry);
        let r22 = fa::mac_with_carry(r22, (self.0).0[10usize], (other.0).0[12usize], &mut carry);
        let r23 = carry;
        let mut carry = 0;
        let r11 = fa::mac_with_carry(r11, (self.0).0[11usize], (other.0).0[0usize], &mut carry);
        let r12 = fa::mac_with_carry(r12, (self.0).0[11usize], (other.0).0[1usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[11usize], (other.0).0[2usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[11usize], (other.0).0[3usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[11usize], (other.0).0[4usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[11usize], (other.0).0[5usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[11usize], (other.0).0[6usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[11usize], (other.0).0[7usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[11usize], (other.0).0[8usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[11usize], (other.0).0[9usize], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[11usize], (other.0).0[10usize], &mut carry);
        let r22 = fa::mac_with_carry(r22, (self.0).0[11usize], (other.0).0[11usize], &mut carry);
        let r23 = fa::mac_with_carry(r23, (self.0).0[11usize], (other.0).0[12usize], &mut carry);
        let r24 = carry;
        let mut carry = 0;
        let r12 = fa::mac_with_carry(r12, (self.0).0[12usize], (other.0).0[0usize], &mut carry);
        let r13 = fa::mac_with_carry(r13, (self.0).0[12usize], (other.0).0[1usize], &mut carry);
        let r14 = fa::mac_with_carry(r14, (self.0).0[12usize], (other.0).0[2usize], &mut carry);
        let r15 = fa::mac_with_carry(r15, (self.0).0[12usize], (other.0).0[3usize], &mut carry);
        let r16 = fa::mac_with_carry(r16, (self.0).0[12usize], (other.0).0[4usize], &mut carry);
        let r17 = fa::mac_with_carry(r17, (self.0).0[12usize], (other.0).0[5usize], &mut carry);
        let r18 = fa::mac_with_carry(r18, (self.0).0[12usize], (other.0).0[6usize], &mut carry);
        let r19 = fa::mac_with_carry(r19, (self.0).0[12usize], (other.0).0[7usize], &mut carry);
        let r20 = fa::mac_with_carry(r20, (self.0).0[12usize], (other.0).0[8usize], &mut carry);
        let r21 = fa::mac_with_carry(r21, (self.0).0[12usize], (other.0).0[9usize], &mut carry);
        let r22 = fa::mac_with_carry(r22, (self.0).0[12usize], (other.0).0[10usize], &mut carry);
        let r23 = fa::mac_with_carry(r23, (self.0).0[12usize], (other.0).0[11usize], &mut carry);
        let r24 = fa::mac_with_carry(r24, (self.0).0[12usize], (other.0).0[12usize], &mut carry);
        let r25 = carry;
        self.mont_reduce(
            r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11, r12, r13, r14, r15, r16, r17, r18, r19, r20, r21, r22,
            r23, r24, r25,
        );
    }
}

impl<'a, P: Fp832Parameters> DivAssign<&'a Self> for Fp832<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}
