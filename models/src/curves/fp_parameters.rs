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

use snarkos_utilities::biginteger::*;

/// A trait that defines parameters for a prime field.
pub trait FpParameters: 'static + Send + Sync + Sized {
    type BigInteger: BigInteger;

    /// The modulus of the field.
    const MODULUS: Self::BigInteger;

    /// The number of bits needed to represent the `Self::MODULUS`.
    const MODULUS_BITS: u32;

    /// The number of bits that must be shaved from the beginning of
    /// the representation when randomly sampling.
    const REPR_SHAVE_BITS: u32;

    /// R = 2^256 % Self::MODULUS
    const R: Self::BigInteger;

    /// R2 = R^2 % Self::MODULUS
    const R2: Self::BigInteger;

    /// INV = -(MODULUS^{-1} mod MODULUS) mod MODULUS
    const INV: u64;

    /// A multiplicative generator that is also a quadratic nonresidue.
    /// `Self::GENERATOR` is an element having multiplicative order
    /// `Self::MODULUS - 1`.
    /// There also does not exist `x` such that `Self::GENERATOR = x^2 %
    /// Self::MODULUS`
    const GENERATOR: Self::BigInteger;

    /// The number of bits that can be reliably stored.
    /// (Should equal `SELF::MODULUS_BITS - 1`)
    const CAPACITY: u32;

    /// 2^s * t = MODULUS - 1 with t odd. This is the two-adicity of the prime.
    const TWO_ADICITY: u32;

    /// 2^s root of unity computed by GENERATOR^t
    const ROOT_OF_UNITY: Self::BigInteger;

    /// t for 2^s * t = MODULUS - 1
    const T: Self::BigInteger;

    /// (t - 1) / 2
    const T_MINUS_ONE_DIV_TWO: Self::BigInteger;

    /// (Self::MODULUS - 1) / 2
    const MODULUS_MINUS_ONE_DIV_TWO: Self::BigInteger;
}
