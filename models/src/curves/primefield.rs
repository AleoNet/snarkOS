use crate::curves::{Field, FpParameters};
use snarkvm_utilities::biginteger::*;

use std::str::FromStr;

/// The interface for a prime field.
pub trait PrimeField: Field + FromStr {
    type Params: FpParameters<BigInt = Self::BigInt>;
    type BigInt: BigInteger;

    /// Returns a prime field element from its underlying representation.
    fn from_repr(repr: <Self::Params as FpParameters>::BigInt) -> Self;

    /// Returns the underlying representation of the prime field element.
    fn into_repr(&self) -> Self::BigInt;

    /// Returns a prime field element from its underlying raw representation.
    fn from_repr_raw(repr: Self::BigInt) -> Self;

    /// Returns the underlying raw representation of the prime field element.
    fn into_repr_raw(&self) -> Self::BigInt;

    /// Returns a field element if the set of bytes forms a valid field element,
    /// otherwise returns None.
    fn from_random_bytes(bytes: &[u8]) -> Option<Self>;

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
        Self::Params::MODULUS_BITS as usize
    }

    /// Returns the trace.
    fn trace() -> Self::BigInt {
        Self::Params::T
    }

    /// Returns the trace minus one divided by two.
    fn trace_minus_one_div_two() -> Self::BigInt {
        Self::Params::T_MINUS_ONE_DIV_TWO
    }

    /// Returns the modulus minus one divided by two.
    fn modulus_minus_one_div_two() -> Self::BigInt {
        Self::Params::MODULUS_MINUS_ONE_DIV_TWO
    }
}
