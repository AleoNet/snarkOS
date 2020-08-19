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

use crate::curves::{One, Zero};
use snarkos_utilities::{
    bititerator::BitIterator,
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    serialize::{
        CanonicalDeserialize,
        CanonicalDeserializeWithFlags,
        CanonicalSerialize,
        CanonicalSerializeWithFlags,
        ConstantSerializedSize,
    },
};

use std::{
    fmt::{Debug, Display},
    hash::Hash,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use serde::{Deserialize, Serialize};

/// The interface for a generic field.
pub trait Field:
    ToBytes
    + FromBytes
    + Copy
    + Clone
    + Debug
    + Display
    + Default
    + Send
    + Sync
    + 'static
    + Eq
    + One
    + Ord
    + Neg<Output = Self>
    + UniformRand
    + Sized
    + Hash
    + From<u128>
    + From<u64>
    + From<u32>
    + From<u16>
    + From<u8>
    + for<'a> Add<&'a Self, Output = Self>
    + for<'a> Sub<&'a Self, Output = Self>
    + for<'a> Mul<&'a Self, Output = Self>
    + for<'a> Div<&'a Self, Output = Self>
    + for<'a> AddAssign<&'a Self>
    + for<'a> SubAssign<&'a Self>
    + for<'a> MulAssign<&'a Self>
    + for<'a> DivAssign<&'a Self>
    + CanonicalSerialize
    + ConstantSerializedSize
    + CanonicalSerializeWithFlags
    + CanonicalDeserialize
    + CanonicalDeserializeWithFlags
    + Serialize
    + for<'a> Deserialize<'a>
    + Zero
{
    /// Returns the characteristic of the field.
    fn characteristic<'a>() -> &'a [u64];

    /// Returns `self + self`.
    #[must_use]
    fn double(&self) -> Self;

    /// Doubles `self` in place.
    fn double_in_place(&mut self) -> &mut Self;

    /// Returns `self * self`.
    #[must_use]
    fn square(&self) -> Self;

    /// Squares `self` in place.
    fn square_in_place(&mut self) -> &mut Self;

    /// Computes the multiplicative inverse of `self` if `self` is nonzero.
    #[must_use]
    fn inverse(&self) -> Option<Self>;

    // Sets `self` to `self`'s inverse if it exists. Otherwise it is a no-op.
    fn inverse_in_place(&mut self) -> Option<&mut Self>;

    /// Exponentiates this element by a power of the base prime modulus via
    /// the Frobenius automorphism.
    fn frobenius_map(&mut self, power: usize);

    /// Exponentiates this element by a number represented with `u64` limbs,
    /// least significant limb first.
    fn pow<S: AsRef<[u64]>>(&self, exp: S) -> Self {
        let mut res = Self::one();

        let mut found_one = false;

        for i in BitIterator::new(exp) {
            if !found_one {
                if i {
                    found_one = true;
                } else {
                    continue;
                }
            }

            res.square_in_place();

            if i {
                res *= self;
            }
        }
        res
    }

    /// Returns a field element if the set of bytes forms a valid field element,
    /// otherwise returns None. This function is primarily intended for sampling
    /// random field elements from a hash-function or RNG output.
    fn from_random_bytes(bytes: &[u8]) -> Option<Self> {
        Self::from_random_bytes_with_flags(bytes).map(|f| f.0)
    }

    /// Returns a field element with an extra sign bit used for group parsing if
    /// the set of bytes forms a valid field element, otherwise returns
    /// None. This function is primarily intended for sampling
    /// random field elements from a hash-function or RNG output.
    fn from_random_bytes_with_flags(bytes: &[u8]) -> Option<(Self, u8)>;
}
