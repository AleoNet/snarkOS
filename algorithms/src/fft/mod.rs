//! This crate implements functions for manipulating polynomials over finite fields,
//! including FFTs.

pub mod domain;
pub use domain::EvaluationDomain;

pub mod evaluations;
pub use evaluations::Evaluations;

pub mod polynomial;
pub use polynomial::{DenseOrSparsePolynomial, DensePolynomial, SparsePolynomial};

pub(crate) mod multicore;

#[cfg(test)]
mod tests;
