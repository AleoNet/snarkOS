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

#![allow(non_snake_case)]

use crate::{
    ahp::{indexer::Matrix, *},
    BTreeMap,
    Cow,
    ToString,
};
use derivative::Derivative;
use snarkos_errors::{gadgets::SynthesisError, serialization::SerializationError};
use snarkos_models::{
    curves::{batch_inversion, Field, PrimeField},
    gadgets::r1cs::{ConstraintSystem, Index as VarIndex, LinearCombination, Variable},
};
use snarkvm_algorithms::{cfg_iter_mut, fft::Evaluations as EvaluationsOnDomain};
use snarkvm_polycommit::LabeledPolynomial;

use snarkos_utilities::serialize::*;

// #[cfg(feature = "parallel")]
// use rayon::prelude::*;

/* ************************************************************************* */
/* ************************************************************************* */
/* ************************************************************************* */

/// Stores constraints during index generation.
pub(crate) struct IndexerConstraintSystem<F: Field> {
    pub(crate) num_input_variables: usize,
    pub(crate) num_witness_variables: usize,
    pub(crate) num_constraints: usize,
    pub(crate) a: Vec<Vec<(F, VarIndex)>>,
    pub(crate) b: Vec<Vec<(F, VarIndex)>>,
    pub(crate) c: Vec<Vec<(F, VarIndex)>>,
}

// This function converts a matrix output by Zexe's constraint infrastructure
// to the one used in this crate.
fn to_matrix_helper<F: Field>(matrix: &[Vec<(F, VarIndex)>], num_input_variables: usize) -> Matrix<F> {
    let mut new_matrix = Vec::with_capacity(matrix.len());
    for row in matrix {
        let mut new_row = Vec::with_capacity(row.len());
        for (fe, column) in row {
            let column = match column {
                VarIndex::Input(i) => *i,
                VarIndex::Aux(i) => num_input_variables + i,
            };
            new_row.push((*fe, column))
        }
        new_matrix.push(new_row)
    }
    new_matrix
}

impl<F: Field> IndexerConstraintSystem<F> {
    #[inline]
    fn make_row(l: &LinearCombination<F>) -> Vec<(F, VarIndex)> {
        l.as_ref()
            .iter()
            .map(|(var, coeff)| (*coeff, var.get_unchecked()))
            .collect()
    }

    pub(crate) fn new() -> Self {
        Self {
            num_input_variables: 1,
            num_witness_variables: 0,
            num_constraints: 0,
            a: Vec::new(),
            b: Vec::new(),
            c: Vec::new(),
        }
    }

    pub(crate) fn a_matrix(&self) -> Vec<Vec<(F, usize)>> {
        to_matrix_helper(&self.a, self.num_input_variables)
    }

    pub(crate) fn b_matrix(&self) -> Vec<Vec<(F, usize)>> {
        to_matrix_helper(&self.b, self.num_input_variables)
    }

    pub(crate) fn c_matrix(&self) -> Vec<Vec<(F, usize)>> {
        to_matrix_helper(&self.c, self.num_input_variables)
    }

    pub(crate) fn num_non_zero(&self) -> usize {
        let a_density = self.a.iter().map(|row| row.len()).sum();
        let b_density = self.b.iter().map(|row| row.len()).sum();
        let c_density = self.c.iter().map(|row| row.len()).sum();

        let max = *[a_density, b_density, c_density]
            .iter()
            .max()
            .expect("iterator is not empty");
        max
    }

    pub(crate) fn make_matrices_square(&mut self) {
        let num_variables = self.num_input_variables + self.num_witness_variables;
        let num_non_zero = self.num_non_zero();
        let matrix_dim = padded_matrix_dim(num_variables, self.num_constraints);
        make_matrices_square(self, num_variables);
        assert_eq!(
            self.num_input_variables + self.num_witness_variables,
            self.num_constraints,
            "padding failed!"
        );
        assert_eq!(
            self.num_input_variables + self.num_witness_variables,
            matrix_dim,
            "padding does not result in expected matrix size!"
        );
        assert_eq!(self.num_non_zero(), num_non_zero, "padding changed matrix density");
    }
}

impl<ConstraintF: Field> ConstraintSystem<ConstraintF> for IndexerConstraintSystem<ConstraintF> {
    type Root = Self;

    #[inline]
    fn alloc<F, A, AR>(&mut self, _: A, _: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<ConstraintF, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        // There is no assignment, so we don't invoke the
        // function for obtaining one.

        let index = self.num_witness_variables;
        self.num_witness_variables += 1;

        Ok(Variable::new_unchecked(VarIndex::Aux(index)))
    }

    #[inline]
    fn alloc_input<F, A, AR>(&mut self, _: A, _: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<ConstraintF, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        // There is no assignment, so we don't invoke the
        // function for obtaining one.

        let index = self.num_input_variables;
        self.num_input_variables += 1;

        Ok(Variable::new_unchecked(VarIndex::Input(index)))
    }

    fn enforce<A, AR, LA, LB, LC>(&mut self, _: A, a: LA, b: LB, c: LC)
    where
        A: FnOnce() -> AR,
        AR: AsRef<str>,
        LA: FnOnce(LinearCombination<ConstraintF>) -> LinearCombination<ConstraintF>,
        LB: FnOnce(LinearCombination<ConstraintF>) -> LinearCombination<ConstraintF>,
        LC: FnOnce(LinearCombination<ConstraintF>) -> LinearCombination<ConstraintF>,
    {
        self.a.push(Self::make_row(&a(LinearCombination::zero())));
        self.b.push(Self::make_row(&b(LinearCombination::zero())));
        self.c.push(Self::make_row(&c(LinearCombination::zero())));

        self.num_constraints += 1;
    }

    fn push_namespace<NR, N>(&mut self, _: N)
    where
        NR: AsRef<str>,
        N: FnOnce() -> NR,
    {
        // Do nothing; we don't care about namespaces in this context.
    }

    fn pop_namespace(&mut self) {
        // Do nothing; we don't care about namespaces in this context.
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn num_constraints(&self) -> usize {
        self.num_constraints
    }
}

/// This must *always* be in sync with `make_matrices_square`.
pub(crate) fn padded_matrix_dim(num_formatted_variables: usize, num_constraints: usize) -> usize {
    core::cmp::max(num_formatted_variables, num_constraints)
}

pub(crate) fn make_matrices_square<F: Field, CS: ConstraintSystem<F>>(cs: &mut CS, num_formatted_variables: usize) {
    let num_constraints = cs.num_constraints();
    let matrix_padding = ((num_formatted_variables as isize) - (num_constraints as isize)).abs();

    if num_formatted_variables > num_constraints {
        use core::convert::identity as iden;
        // Add dummy constraints of the form 0 * 0 == 0
        for i in 0..matrix_padding {
            cs.enforce(|| format!("pad constraint {}", i), iden, iden, iden);
        }
    } else {
        // Add dummy unconstrained variables
        for i in 0..matrix_padding {
            let _ = cs
                .alloc(|| format!("pad var {}", i), || Ok(F::one()))
                .expect("alloc failed");
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "F: PrimeField"))]
#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct MatrixEvals<'a, F: PrimeField> {
    /// Evaluations of the LDE of row.
    pub row: Cow<'a, EvaluationsOnDomain<F>>,
    /// Evaluations of the LDE of col.
    pub col: Cow<'a, EvaluationsOnDomain<F>>,
    /// Evaluations of the LDE of val.
    pub val: Cow<'a, EvaluationsOnDomain<F>>,
}

/// Contains information about the arithmetization of the matrix M^*.
/// Here `M^*(i, j) := M(j, i) * u_H(j, j)`. For more details, see [COS19].
#[derive(Derivative)]
#[derivative(Clone(bound = "F: PrimeField"))]
#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct MatrixArithmetization<'a, F: PrimeField> {
    /// LDE of the row indices of M^*.
    pub row: LabeledPolynomial<'a, F>,
    /// LDE of the column indices of M^*.
    pub col: LabeledPolynomial<'a, F>,
    /// LDE of the non-zero entries of M^*.
    pub val: LabeledPolynomial<'a, F>,
    /// LDE of the vector containing entry-wise products of `row` and `col`,
    /// where `row` and `col` are as above.
    pub row_col: LabeledPolynomial<'a, F>,

    /// Evaluation of `self.row`, `self.col`, and `self.val` on the domain `K`.
    pub evals_on_K: MatrixEvals<'a, F>,

    /// Evaluation of `self.row`, `self.col`, and, `self.val` on
    /// an extended domain B (of size > `3K`).
    // TODO: rename B everywhere.
    pub evals_on_B: MatrixEvals<'a, F>,

    /// Evaluation of `self.row_col` on an extended domain B (of size > `3K`).
    pub row_col_evals_on_B: Cow<'a, EvaluationsOnDomain<F>>,
}

// TODO for debugging: add test that checks result of arithmetize_matrix(M).
pub(crate) fn arithmetize_matrix<'a, F: PrimeField>(
    matrix_name: &str,
    matrix: &mut Matrix<F>,
    interpolation_domain: EvaluationDomain<F>,
    output_domain: EvaluationDomain<F>,
    input_domain: EvaluationDomain<F>,
    expanded_domain: EvaluationDomain<F>,
) -> MatrixArithmetization<'a, F> {
    let matrix_time = start_timer!(|| "Computing row, col, and val LDEs");

    let elems: Vec<_> = output_domain.elements().collect();

    let vec_len: usize = matrix.iter().map(|row| row.len()).sum();
    let mut row_vec = Vec::with_capacity(vec_len);
    let mut col_vec = Vec::with_capacity(vec_len);
    let mut val_vec = Vec::with_capacity(vec_len);

    let eq_poly_vals_time = start_timer!(|| "Precomputing eq_poly_vals");
    let eq_poly_vals: BTreeMap<F, F> = output_domain
        .elements()
        .zip(output_domain.batch_eval_unnormalized_bivariate_lagrange_poly_with_same_inputs())
        .collect();
    end_timer!(eq_poly_vals_time);

    let lde_evals_time = start_timer!(|| "Computing row, col and val evals");
    let mut inverses = Vec::with_capacity(vec_len);

    let mut count = 0;

    // Recall that we are computing the arithmetization of M^*,
    // where `M^*(i, j) := M(j, i) * u_H(j, j)`.
    for (r, row) in matrix.iter_mut().enumerate() {
        if !is_in_ascending_order(&row, |(_, a), (_, b)| a < b) {
            row.sort_by(|(_, a), (_, b)| a.cmp(b));
        };

        for &mut (val, i) in row {
            let row_val = elems[r];
            let col_val = elems[output_domain.reindex_by_subdomain(input_domain, i)];

            // We are dealing with the transpose of M
            row_vec.push(col_val);
            col_vec.push(row_val);
            val_vec.push(val);
            inverses.push(eq_poly_vals[&col_val]);

            count += 1;
        }
    }
    batch_inversion::<F>(&mut inverses);

    cfg_iter_mut!(val_vec).zip(inverses).for_each(|(v, inv)| *v *= &inv);
    end_timer!(lde_evals_time);

    for _ in 0..(interpolation_domain.size() - count) {
        col_vec.push(elems[0]);
        row_vec.push(elems[0]);
        val_vec.push(F::zero());
    }
    let row_col_vec: Vec<_> = row_vec.iter().zip(&col_vec).map(|(row, col)| *row * col).collect();

    let interpolate_time = start_timer!(|| "Interpolating on K and B");
    let row_evals_on_K = EvaluationsOnDomain::from_vec_and_domain(row_vec, interpolation_domain);
    let col_evals_on_K = EvaluationsOnDomain::from_vec_and_domain(col_vec, interpolation_domain);
    let val_evals_on_K = EvaluationsOnDomain::from_vec_and_domain(val_vec, interpolation_domain);
    let row_col_evals_on_K = EvaluationsOnDomain::from_vec_and_domain(row_col_vec, interpolation_domain);

    let row = row_evals_on_K.clone().interpolate();
    let col = col_evals_on_K.clone().interpolate();
    let val = val_evals_on_K.clone().interpolate();
    let row_col = row_col_evals_on_K.interpolate();

    let row_evals_on_B = EvaluationsOnDomain::from_vec_and_domain(expanded_domain.fft(&row), expanded_domain);
    let col_evals_on_B = EvaluationsOnDomain::from_vec_and_domain(expanded_domain.fft(&col), expanded_domain);
    let val_evals_on_B = EvaluationsOnDomain::from_vec_and_domain(expanded_domain.fft(&val), expanded_domain);
    let row_col_evals_on_B = EvaluationsOnDomain::from_vec_and_domain(expanded_domain.fft(&row_col), expanded_domain);
    end_timer!(interpolate_time);

    end_timer!(matrix_time);
    let evals_on_K = MatrixEvals {
        row: Cow::Owned(row_evals_on_K),
        col: Cow::Owned(col_evals_on_K),
        val: Cow::Owned(val_evals_on_K),
    };
    let evals_on_B = MatrixEvals {
        row: Cow::Owned(row_evals_on_B),
        col: Cow::Owned(col_evals_on_B),
        val: Cow::Owned(val_evals_on_B),
    };

    let m_name = matrix_name.to_string();
    MatrixArithmetization {
        row: LabeledPolynomial::new_owned(m_name.clone() + "_row", row, None, None),
        col: LabeledPolynomial::new_owned(m_name.clone() + "_col", col, None, None),
        val: LabeledPolynomial::new_owned(m_name.clone() + "_val", val, None, None),
        row_col: LabeledPolynomial::new_owned(m_name + "_row_col", row_col, None, None),
        evals_on_K,
        evals_on_B,
        row_col_evals_on_B: Cow::Owned(row_col_evals_on_B),
    }
}

fn is_in_ascending_order<T: Ord>(x_s: &[T], is_less_than: impl Fn(&T, &T) -> bool) -> bool {
    if x_s.is_empty() {
        true
    } else {
        let mut i = 0;
        let mut is_sorted = true;
        while i < (x_s.len() - 1) {
            is_sorted &= is_less_than(&x_s[i], &x_s[i + 1]);
            i += 1;
        }
        is_sorted
    }
}

/* ************************************************************************* */
/* ************************************************************************* */
/* ************************************************************************* */

pub(crate) struct ProverConstraintSystem<F: Field> {
    // Assignments of variables
    pub(crate) input_assignment: Vec<F>,
    pub(crate) witness_assignment: Vec<F>,
    pub(crate) num_input_variables: usize,
    pub(crate) num_witness_variables: usize,
    pub(crate) num_constraints: usize,
}

impl<F: Field> ProverConstraintSystem<F> {
    pub(crate) fn new() -> Self {
        Self {
            input_assignment: vec![F::one()],
            witness_assignment: Vec::new(),
            num_input_variables: 1usize,
            num_witness_variables: 0usize,
            num_constraints: 0usize,
        }
    }

    /// Formats the public input according to the requirements of the constraint
    /// system
    pub(crate) fn format_public_input(public_input: &[F]) -> Vec<F> {
        let mut input = vec![F::one()];
        input.extend_from_slice(public_input);
        input
    }

    /// Takes in a previously formatted public input and removes the formatting
    /// imposed by the constraint system.
    pub(crate) fn unformat_public_input(input: &[F]) -> Vec<F> {
        input[1..].to_vec()
    }

    pub(crate) fn make_matrices_square(&mut self) {
        let num_variables = self.num_input_variables + self.num_witness_variables;
        make_matrices_square(self, num_variables);
        assert_eq!(
            self.num_input_variables + self.num_witness_variables,
            self.num_constraints,
            "padding failed!"
        );
    }
}

impl<ConstraintF: Field> ConstraintSystem<ConstraintF> for ProverConstraintSystem<ConstraintF> {
    type Root = Self;

    #[inline]
    fn alloc<F, A, AR>(&mut self, _: A, f: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<ConstraintF, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.num_witness_variables;
        self.num_witness_variables += 1;

        self.witness_assignment.push(f()?);
        Ok(Variable::new_unchecked(VarIndex::Aux(index)))
    }

    #[inline]
    fn alloc_input<F, A, AR>(&mut self, _: A, f: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<ConstraintF, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.num_input_variables;
        self.num_input_variables += 1;

        self.input_assignment.push(f()?);
        Ok(Variable::new_unchecked(VarIndex::Input(index)))
    }

    #[inline]
    fn enforce<A, AR, LA, LB, LC>(&mut self, _: A, _: LA, _: LB, _: LC)
    where
        A: FnOnce() -> AR,
        AR: AsRef<str>,
        LA: FnOnce(LinearCombination<ConstraintF>) -> LinearCombination<ConstraintF>,
        LB: FnOnce(LinearCombination<ConstraintF>) -> LinearCombination<ConstraintF>,
        LC: FnOnce(LinearCombination<ConstraintF>) -> LinearCombination<ConstraintF>,
    {
        self.num_constraints += 1;
    }

    fn push_namespace<NR, N>(&mut self, _: N)
    where
        NR: AsRef<str>,
        N: FnOnce() -> NR,
    {
        // Do nothing; we don't care about namespaces in this context.
    }

    fn pop_namespace(&mut self) {
        // Do nothing; we don't care about namespaces in this context.
    }

    fn get_root(&mut self) -> &mut Self::Root {
        self
    }

    fn num_constraints(&self) -> usize {
        self.num_constraints
    }
}
