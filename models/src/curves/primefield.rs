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

use crate::curves::{Field, FpParameters};
use snarkos_utilities::biginteger::BigInteger;

use std::str::FromStr;

/// The interface for a prime field.
pub trait PrimeField: Field + FromStr {
    type Parameters: FpParameters<BigInteger = Self::BigInteger>;
    type BigInteger: BigInteger;

    /// Returns a prime field element from its underlying representation.
    fn from_repr(repr: Self::BigInteger) -> Option<Self>;

    /// Returns the underlying representation of the prime field element.
    fn into_repr(&self) -> Self::BigInteger;

    /// Returns a prime field element from its underlying raw representation.
    fn from_repr_raw(repr: Self::BigInteger) -> Self;

    /// Returns the underlying raw representation of the prime field element.
    fn into_repr_raw(&self) -> Self::BigInteger;

    /// Returns the multiplicative generator of `char()` - 1 order.
    fn multiplicative_generator() -> Self;

    /// Returns the 2^s root of unity.
    fn root_of_unity() -> Self;

    /// Return the a QNR^T
    fn qnr_to_t() -> Self {
        Self::root_of_unity()
    }

    /// Returns the field size in bits.
    fn size_in_bits() -> usize {
        Self::Parameters::MODULUS_BITS as usize
    }

    /// Returns the trace.
    fn trace() -> Self::BigInteger {
        Self::Parameters::T
    }

    /// Returns the trace minus one divided by two.
    fn trace_minus_one_div_two() -> Self::BigInteger {
        Self::Parameters::T_MINUS_ONE_DIV_TWO
    }

    /// Returns the modulus minus one divided by two.
    fn modulus_minus_one_div_two() -> Self::BigInteger {
        Self::Parameters::MODULUS_MINUS_ONE_DIV_TWO
    }
}
