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

use crate::{Cow, String, Vec};
use core::{
    borrow::Borrow,
    fmt::Debug,
    ops::{AddAssign, MulAssign, SubAssign},
};
use rand_core::RngCore;
pub use snarkos_algorithms::fft::DensePolynomial as Polynomial;
use snarkos_errors::serialization::SerializationError;
use snarkos_models::curves::Field;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    error as error_fn,
    serialize::*,
};

/// Labels a `LabeledPolynomial` or a `LabeledCommitment`.
pub type PolynomialLabel = String;

/// Defines the minimal interface for public params for any polynomial
/// commitment scheme.
pub trait PCUniversalParams: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + ToBytes + FromBytes {
    /// Outputs the maximum degree supported by the committer key.
    fn max_degree(&self) -> usize;
}

/// Defines the minimal interface of committer keys for any polynomial
/// commitment scheme.
pub trait PCCommitterKey: CanonicalSerialize + CanonicalDeserialize + Clone + Debug {
    /// Outputs the maximum degree supported by the universal parameters
    /// `Self` was derived from.
    fn max_degree(&self) -> usize;

    /// Outputs the maximum degree supported by the committer key.
    fn supported_degree(&self) -> usize;
}

/// Defines the minimal interface of verifier keys for any polynomial
/// commitment scheme.
pub trait PCVerifierKey: CanonicalSerialize + CanonicalDeserialize + Clone + Debug {
    /// Outputs the maximum degree supported by the universal parameters
    /// `Self` was derived from.
    fn max_degree(&self) -> usize;

    /// Outputs the maximum degree supported by the verifier key.
    fn supported_degree(&self) -> usize;
}

/// Defines the minimal interface of commitments for any polynomial
/// commitment scheme.
pub trait PCCommitment: CanonicalDeserialize + CanonicalSerialize + Clone + Debug + ToBytes {
    /// Outputs a non-hiding commitment to the zero polynomial.
    fn empty() -> Self;

    /// Does this commitment have a degree bound?
    fn has_degree_bound(&self) -> bool;
}

/// Defines the minimal interface of commitment randomness for any polynomial
/// commitment scheme.
pub trait PCRandomness: CanonicalSerialize + CanonicalDeserialize + Clone {
    /// Outputs empty randomness that does not hide the commitment.
    fn empty() -> Self;

    /// Samples randomness for commitments;
    /// `num_queries` specifies the number of queries that the commitment will be opened at.
    /// `has_degree_bound` indicates that the corresponding commitment has an enforced
    /// strict degree bound.
    fn rand<R: RngCore>(num_queries: usize, has_degree_bound: bool, rng: &mut R) -> Self;
}

/// Defines the minimal interface of evaluation proofs for any polynomial
/// commitment scheme.
pub trait PCProof: CanonicalSerialize + CanonicalDeserialize + Clone + ToBytes {}

/// A polynomial along with information about its degree bound (if any), and the
/// maximum number of queries that will be made to it. This latter number determines
/// the amount of protection that will be provided to a commitment for this polynomial.
#[derive(Debug, Clone, CanonicalSerialize, CanonicalDeserialize)]
pub struct LabeledPolynomial<'a, F: Field> {
    label: PolynomialLabel,
    polynomial: Cow<'a, Polynomial<F>>,
    degree_bound: Option<usize>,
    hiding_bound: Option<usize>,
}

impl<'a, F: Field> core::ops::Deref for LabeledPolynomial<'a, F> {
    type Target = Polynomial<F>;

    fn deref(&self) -> &Self::Target {
        &self.polynomial
    }
}

impl<'a, F: Field> LabeledPolynomial<'a, F> {
    /// Construct a new labeled polynomial by consuming `polynomial`.
    pub fn new_owned(
        label: PolynomialLabel,
        polynomial: Polynomial<F>,
        degree_bound: Option<usize>,
        hiding_bound: Option<usize>,
    ) -> Self {
        Self {
            label,
            polynomial: Cow::Owned(polynomial),
            degree_bound,

            hiding_bound,
        }
    }

    /// Construct a new labeled polynomial.
    pub fn new(
        label: PolynomialLabel,
        polynomial: &'a Polynomial<F>,
        degree_bound: Option<usize>,
        hiding_bound: Option<usize>,
    ) -> Self {
        Self {
            label,
            polynomial: Cow::Borrowed(polynomial),
            degree_bound,
            hiding_bound,
        }
    }

    /// Return the label for `self`.
    pub fn label(&self) -> &String {
        &self.label
    }

    /// Retrieve the polynomial from `self`.
    pub fn polynomial(&self) -> &Polynomial<F> {
        &self.polynomial
    }

    /// Evaluate the polynomial in `self`.
    pub fn evaluate(&self, point: F) -> F {
        self.polynomial.evaluate(point)
    }

    /// Retrieve the degree bound in `self`.
    pub fn degree_bound(&self) -> Option<usize> {
        self.degree_bound
    }

    /// Retrieve whether the polynomial in `self` should be hidden.
    pub fn is_hiding(&self) -> bool {
        self.hiding_bound.is_some()
    }

    /// Retrieve the hiding bound for the polynomial in `self`.
    pub fn hiding_bound(&self) -> Option<usize> {
        self.hiding_bound
    }
}

/// A commitment along with information about its degree bound (if any).
#[derive(Clone, CanonicalSerialize)]
pub struct LabeledCommitment<C: PCCommitment> {
    label: PolynomialLabel,
    commitment: C,
    degree_bound: Option<usize>,
}

// NOTE: Serializing the LabeledCommitments struct is done by serializing
// _WITHOUT_ the labels or the degree bound. Deserialization is _NOT_ supported,
// and you should construct the struct via the `LabeledCommitment::new` method after
// deserializing the Commitment.
impl<C: PCCommitment> ToBytes for LabeledCommitment<C> {
    fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
        CanonicalSerialize::serialize(&self.commitment, &mut writer).map_err(|_| error_fn("could not serialize struct"))
    }
}

impl<C: PCCommitment> LabeledCommitment<C> {
    /// Instantiate a new polynomial_context.
    pub fn new(label: PolynomialLabel, commitment: C, degree_bound: Option<usize>) -> Self {
        Self {
            label,
            commitment,
            degree_bound,
        }
    }

    /// Return the label for `self`.
    pub fn label(&self) -> &String {
        &self.label
    }

    /// Retrieve the commitment from `self`.
    pub fn commitment(&self) -> &C {
        &self.commitment
    }

    /// Retrieve the degree bound in `self`.
    pub fn degree_bound(&self) -> Option<usize> {
        self.degree_bound
    }
}

/// A term in a linear combination.
#[derive(Hash, Ord, PartialOrd, Clone, Eq, PartialEq, Debug)]
pub enum LCTerm {
    /// The constant term representing `one`.
    One,
    /// Label for a polynomial.
    PolyLabel(String),
}

impl LCTerm {
    /// Returns `true` if `self == LCTerm::One`
    #[inline]
    pub fn is_one(&self) -> bool {
        if let LCTerm::One = self { true } else { false }
    }
}

impl From<PolynomialLabel> for LCTerm {
    fn from(other: PolynomialLabel) -> Self {
        Self::PolyLabel(other)
    }
}

impl<'a> From<&'a str> for LCTerm {
    fn from(other: &str) -> Self {
        Self::PolyLabel(other.into())
    }
}

impl core::convert::TryInto<PolynomialLabel> for LCTerm {
    type Error = ();

    fn try_into(self) -> Result<PolynomialLabel, ()> {
        match self {
            Self::One => Err(()),
            Self::PolyLabel(l) => Ok(l),
        }
    }
}

impl<'a> core::convert::TryInto<&'a PolynomialLabel> for &'a LCTerm {
    type Error = ();

    fn try_into(self) -> Result<&'a PolynomialLabel, ()> {
        match self {
            LCTerm::One => Err(()),
            LCTerm::PolyLabel(l) => Ok(l),
        }
    }
}

impl<B: Borrow<String>> PartialEq<B> for LCTerm {
    fn eq(&self, other: &B) -> bool {
        match self {
            Self::One => false,
            Self::PolyLabel(l) => l == other.borrow(),
        }
    }
}

/// A labeled linear combinations of polynomials.
#[derive(Clone, Debug)]
pub struct LinearCombination<F> {
    /// The label.
    pub label: String,
    /// The linear combination of `(coeff, poly_label)` pairs.
    terms: Vec<(F, LCTerm)>,
}

impl<F: Field> LinearCombination<F> {
    /// Construct an empty labeled linear combination.
    pub fn empty(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            terms: Vec::new(),
        }
    }

    /// Construct a new labeled linear combination.
    /// with the terms specified in `term`.
    pub fn new(label: impl Into<String>, terms: Vec<(F, impl Into<LCTerm>)>) -> Self {
        let terms = terms.into_iter().map(|(c, t)| (c, t.into())).collect();
        Self {
            label: label.into(),
            terms,
        }
    }

    /// Returns the label of the linear combination.
    pub fn label(&self) -> &String {
        &self.label
    }

    /// Returns `true` if the linear combination has no terms.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Add a term to the linear combination.
    pub fn push(&mut self, term: (F, LCTerm)) -> &mut Self {
        self.terms.push(term);
        self
    }
}

impl<'a, F: Field> AddAssign<(F, &'a LinearCombination<F>)> for LinearCombination<F> {
    #[allow(clippy::suspicious_op_assign_impl)]
    fn add_assign(&mut self, (coeff, other): (F, &'a LinearCombination<F>)) {
        self.terms
            .extend(other.terms.iter().map(|(c, t)| (coeff * c, t.clone())));
    }
}

impl<'a, F: Field> SubAssign<(F, &'a LinearCombination<F>)> for LinearCombination<F> {
    fn sub_assign(&mut self, (coeff, other): (F, &'a LinearCombination<F>)) {
        self.terms
            .extend(other.terms.iter().map(|(c, t)| (-coeff * c, t.clone())));
    }
}

impl<'a, F: Field> AddAssign<&'a LinearCombination<F>> for LinearCombination<F> {
    fn add_assign(&mut self, other: &'a LinearCombination<F>) {
        self.terms.extend(other.terms.iter().cloned());
    }
}

impl<'a, F: Field> SubAssign<&'a LinearCombination<F>> for LinearCombination<F> {
    fn sub_assign(&mut self, other: &'a LinearCombination<F>) {
        self.terms.extend(other.terms.iter().map(|(c, t)| (-*c, t.clone())));
    }
}

impl<F: Field> AddAssign<F> for LinearCombination<F> {
    fn add_assign(&mut self, coeff: F) {
        self.terms.push((coeff, LCTerm::One));
    }
}

impl<F: Field> SubAssign<F> for LinearCombination<F> {
    fn sub_assign(&mut self, coeff: F) {
        self.terms.push((-coeff, LCTerm::One));
    }
}

impl<F: Field> MulAssign<F> for LinearCombination<F> {
    fn mul_assign(&mut self, coeff: F) {
        self.terms.iter_mut().for_each(|(c, _)| *c *= &coeff);
    }
}

impl<F: Field> core::ops::Deref for LinearCombination<F> {
    type Target = [(F, LCTerm)];

    fn deref(&self) -> &Self::Target {
        &self.terms
    }
}

/// Helper macro to forward all derived implementations to the ToBytes and FromBytes traits
#[macro_export]
macro_rules! impl_bytes {
    ($ty: ident) => {
        impl<E: PairingEngine> FromBytes for $ty<E> {
            fn read<R: Read>(mut reader: R) -> io::Result<Self> {
                CanonicalDeserialize::deserialize(&mut reader).map_err(|_| error("could not deserialize struct"))
            }
        }

        impl<E: PairingEngine> ToBytes for $ty<E> {
            fn write<W: Write>(&self, mut writer: W) -> io::Result<()> {
                CanonicalSerialize::serialize(self, &mut writer).map_err(|_| error("could not serialize struct"))
            }
        }
    };
}
