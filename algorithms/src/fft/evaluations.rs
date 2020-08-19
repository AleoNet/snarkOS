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

//! A polynomial represented in evaluations form.

use crate::fft::{DensePolynomial, EvaluationDomain};
use snarkos_errors::serialization::SerializationError;
use snarkos_models::curves::PrimeField;
use snarkos_utilities::serialize::*;

use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// Stores a polynomial in evaluation form.
#[derive(Clone, PartialEq, Eq, Hash, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct Evaluations<F: PrimeField> {
    /// The evaluations of a polynomial over the domain `D`
    pub evals: Vec<F>,
    #[doc(hidden)]
    domain: EvaluationDomain<F>,
}

impl<F: PrimeField> Evaluations<F> {
    /// Construct `Self` from evaluations and a domain.
    pub fn from_vec_and_domain(evals: Vec<F>, domain: EvaluationDomain<F>) -> Self {
        Self { evals, domain }
    }

    /// Interpolate a polynomial from a list of evaluations
    pub fn interpolate_by_ref(&self) -> DensePolynomial<F> {
        DensePolynomial::from_coefficients_vec(self.domain.ifft(&self.evals))
    }

    /// Interpolate a polynomial from a list of evaluations
    pub fn interpolate(self) -> DensePolynomial<F> {
        let Self { mut evals, domain } = self;
        domain.ifft_in_place(&mut evals);
        DensePolynomial::from_coefficients_vec(evals)
    }
}

impl<F: PrimeField> std::ops::Index<usize> for Evaluations<F> {
    type Output = F;

    fn index(&self, index: usize) -> &F {
        &self.evals[index]
    }
}

impl<'a, 'b, F: PrimeField> Mul<&'a Evaluations<F>> for &'b Evaluations<F> {
    type Output = Evaluations<F>;

    #[inline]
    fn mul(self, other: &'a Evaluations<F>) -> Evaluations<F> {
        let mut result = self.clone();
        result *= other;
        result
    }
}

impl<'a, F: PrimeField> MulAssign<&'a Evaluations<F>> for Evaluations<F> {
    #[inline]
    fn mul_assign(&mut self, other: &'a Evaluations<F>) {
        assert_eq!(self.domain, other.domain, "domains are unequal");
        self.evals.iter_mut().zip(&other.evals).for_each(|(a, b)| *a *= b);
    }
}

impl<'a, 'b, F: PrimeField> Add<&'a Evaluations<F>> for &'b Evaluations<F> {
    type Output = Evaluations<F>;

    #[inline]
    fn add(self, other: &'a Evaluations<F>) -> Evaluations<F> {
        let mut result = self.clone();
        result += other;
        result
    }
}

impl<'a, F: PrimeField> AddAssign<&'a Evaluations<F>> for Evaluations<F> {
    #[inline]
    fn add_assign(&mut self, other: &'a Evaluations<F>) {
        assert_eq!(self.domain, other.domain, "domains are unequal");
        self.evals.iter_mut().zip(&other.evals).for_each(|(a, b)| *a += b);
    }
}

impl<'a, 'b, F: PrimeField> Sub<&'a Evaluations<F>> for &'b Evaluations<F> {
    type Output = Evaluations<F>;

    #[inline]
    fn sub(self, other: &'a Evaluations<F>) -> Evaluations<F> {
        let mut result = self.clone();
        result -= other;
        result
    }
}

impl<'a, F: PrimeField> SubAssign<&'a Evaluations<F>> for Evaluations<F> {
    #[inline]
    fn sub_assign(&mut self, other: &'a Evaluations<F>) {
        assert_eq!(self.domain, other.domain, "domains are unequal");
        self.evals.iter_mut().zip(&other.evals).for_each(|(a, b)| *a -= b);
    }
}

impl<'a, 'b, F: PrimeField> Div<&'a Evaluations<F>> for &'b Evaluations<F> {
    type Output = Evaluations<F>;

    #[inline]
    fn div(self, other: &'a Evaluations<F>) -> Evaluations<F> {
        let mut result = self.clone();
        result /= other;
        result
    }
}

impl<'a, F: PrimeField> DivAssign<&'a Evaluations<F>> for Evaluations<F> {
    #[inline]
    fn div_assign(&mut self, other: &'a Evaluations<F>) {
        assert_eq!(self.domain, other.domain, "domains are unequal");
        self.evals.iter_mut().zip(&other.evals).for_each(|(a, b)| *a /= b);
    }
}
