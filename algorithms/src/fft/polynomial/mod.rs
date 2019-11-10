//! Work with sparse and dense polynomials.

use crate::fft::{EvaluationDomain, Evaluations};
use snarkos_models::curves::{Field, PrimeField};

use std::{borrow::Cow, convert::TryInto};

use DenseOrSparsePolynomial::*;

mod dense;
pub use dense::DensePolynomial;

mod sparse;
pub use sparse::SparsePolynomial;

/// Represents either a sparse polynomial or a dense one.
#[derive(Clone)]
pub enum DenseOrSparsePolynomial<'a, F: 'a + Field> {
    /// Represents the case where `self` is a sparse polynomial
    SPolynomial(Cow<'a, SparsePolynomial<F>>),
    /// Represents the case where `self` is a dense polynomial
    DPolynomial(Cow<'a, DensePolynomial<F>>),
}

impl<F: Field> From<DensePolynomial<F>> for DenseOrSparsePolynomial<'_, F> {
    fn from(other: DensePolynomial<F>) -> Self {
        DPolynomial(Cow::Owned(other))
    }
}

impl<'a, F: 'a + Field> From<&'a DensePolynomial<F>> for DenseOrSparsePolynomial<'a, F> {
    fn from(other: &'a DensePolynomial<F>) -> Self {
        DPolynomial(Cow::Borrowed(other))
    }
}

impl<F: Field> From<SparsePolynomial<F>> for DenseOrSparsePolynomial<'_, F> {
    fn from(other: SparsePolynomial<F>) -> Self {
        SPolynomial(Cow::Owned(other))
    }
}

impl<'a, F: Field> From<&'a SparsePolynomial<F>> for DenseOrSparsePolynomial<'a, F> {
    fn from(other: &'a SparsePolynomial<F>) -> Self {
        SPolynomial(Cow::Borrowed(other))
    }
}

impl<F: Field> Into<DensePolynomial<F>> for DenseOrSparsePolynomial<'_, F> {
    fn into(self) -> DensePolynomial<F> {
        match self {
            DPolynomial(p) => p.into_owned(),
            SPolynomial(p) => p.into_owned().into(),
        }
    }
}

impl<F: Field> TryInto<SparsePolynomial<F>> for DenseOrSparsePolynomial<'_, F> {
    type Error = ();

    fn try_into(self) -> Result<SparsePolynomial<F>, ()> {
        match self {
            SPolynomial(p) => Ok(p.into_owned()),
            _ => Err(()),
        }
    }
}

impl<F: Field> DenseOrSparsePolynomial<'_, F> {
    /// Checks if the given polynomial is zero.
    pub fn is_zero(&self) -> bool {
        match self {
            SPolynomial(s) => s.is_zero(),
            DPolynomial(d) => d.is_zero(),
        }
    }

    /// Return the degree of `self.
    pub fn degree(&self) -> usize {
        match self {
            SPolynomial(s) => s.degree(),
            DPolynomial(d) => d.degree(),
        }
    }

    #[inline]
    fn leading_coefficient(&self) -> Option<&F> {
        match self {
            SPolynomial(p) => p.coeffs.last().map(|(_, c)| c),
            DPolynomial(p) => p.last(),
        }
    }

    #[inline]
    fn iter_with_index<'a>(&'a self) -> Vec<(usize, F)> {
        match self {
            SPolynomial(p) => p.coeffs.iter().cloned().collect(),
            DPolynomial(p) => p.iter().cloned().enumerate().collect(),
        }
    }

    /// Divide self by another (sparse or dense) polynomial, and returns the quotient and remainder.
    pub fn divide_with_q_and_r(&self, divisor: &Self) -> Option<(DensePolynomial<F>, DensePolynomial<F>)> {
        if self.is_zero() {
            Some((DensePolynomial::zero(), DensePolynomial::zero()))
        } else if divisor.is_zero() {
            panic!("Dividing by zero polynomial")
        } else if self.degree() < divisor.degree() {
            Some((DensePolynomial::zero(), self.clone().into()))
        } else {
            // Now we know that self.degree() >= divisor.degree();
            let mut quotient = vec![F::zero(); self.degree() - divisor.degree() + 1];
            let mut remainder: DensePolynomial<F> = self.clone().into();
            // Can unwrap here because we know self is not zero.
            let divisor_leading_inv = divisor.leading_coefficient().unwrap().inverse().unwrap();
            while !remainder.is_zero() && remainder.degree() >= divisor.degree() {
                let cur_q_coeff = *remainder.coeffs.last().unwrap() * &divisor_leading_inv;
                let cur_q_degree = remainder.degree() - divisor.degree();
                quotient[cur_q_degree] = cur_q_coeff;

                for (i, div_coeff) in divisor.iter_with_index() {
                    remainder[cur_q_degree + i] -= &(cur_q_coeff * &div_coeff);
                }
                while let Some(true) = remainder.coeffs.last().map(|c| c.is_zero()) {
                    remainder.coeffs.pop();
                }
            }
            Some((DensePolynomial::from_coefficients_vec(quotient), remainder))
        }
    }
}
impl<F: PrimeField> DenseOrSparsePolynomial<'_, F> {
    /// Construct `Evaluations` by evaluating a polynomial over the domain `domain`.
    pub fn evaluate_over_domain(poly: impl Into<Self>, domain: EvaluationDomain<F>) -> Evaluations<F> {
        let poly = poly.into();
        poly.eval_over_domain_helper(domain)
    }

    fn eval_over_domain_helper(self, domain: EvaluationDomain<F>) -> Evaluations<F> {
        match self {
            SPolynomial(Cow::Borrowed(s)) => {
                let evals = domain.elements().map(|elem| s.evaluate(elem)).collect();
                Evaluations::from_vec_and_domain(evals, domain)
            }
            SPolynomial(Cow::Owned(s)) => {
                let evals = domain.elements().map(|elem| s.evaluate(elem)).collect();
                Evaluations::from_vec_and_domain(evals, domain)
            }
            DPolynomial(Cow::Borrowed(d)) => Evaluations::from_vec_and_domain(domain.fft(&d.coeffs), domain),
            DPolynomial(Cow::Owned(mut d)) => {
                domain.fft_in_place(&mut d.coeffs);
                Evaluations::from_vec_and_domain(d.coeffs, domain)
            }
        }
    }
}
