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
    curves::PrimeField,
    gadgets::{
        r1cs::ConstraintSystem,
        utilities::{
            arithmetic::{Add, Neg, Sub},
            int::Int64,
        },
    },
};
use snarkos_errors::gadgets::SignedIntegerError;

macro_rules! sub_int_impl {
    ($($gadget: ident)*) => ($(
        impl<F: PrimeField> Sub<F> for $gadget {
            type ErrorType = SignedIntegerError;

            fn sub<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, Self::ErrorType> {
                // Negate other
                let other_neg = other.neg(cs.ns(|| format!("negate")))?;

                // self + negated other
                self.add(cs.ns(|| format!("add_complement")), &other_neg)
            }
        }
    )*)
}

sub_int_impl!(Int64);
