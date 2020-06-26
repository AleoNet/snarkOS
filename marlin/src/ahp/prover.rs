#![allow(non_snake_case)]

use crate::ahp::{constraint_systems::ProverConstraintSystem, indexer::*, verifier::*, *};

use crate::{ToString, Vec};
use core::marker::PhantomData;
use poly_commit::{LabeledPolynomial, Polynomial};
use rand_core::RngCore;
use snarkos_algorithms::{
    cfg_into_iter,
    cfg_iter,
    cfg_iter_mut,
    fft::{EvaluationDomain, Evaluations as EvaluationsOnDomain},
};
use snarkos_errors::{gadgets::SynthesisError, serialization::SerializationError};
use snarkos_models::{
    curves::{batch_inversion, Field, PrimeField},
    gadgets::r1cs::ConstraintSynthesizer,
};
use snarkos_utilities::{bytes::ToBytes, error, serialize::*};
use std::io::Write;

/// State for the AHP prover.
pub struct ProverState<'a, 'b, F: PrimeField, C> {
    formatted_input_assignment: Vec<F>,
    witness_assignment: Vec<F>,
    /// Az
    z_a: Option<Vec<F>>,
    /// Bz
    z_b: Option<Vec<F>>,
    /// query bound b
    zk_bound: usize,

    w_poly: Option<LabeledPolynomial<'b, F>>,
    mz_polys: Option<(LabeledPolynomial<'b, F>, LabeledPolynomial<'b, F>)>,

    index: &'a Index<'a, F, C>,

    /// the random values sent by the verifier in the first round
    verifier_first_msg: Option<VerifierFirstMsg<F>>,

    /// the blinding polynomial for the first round
    mask_poly: Option<LabeledPolynomial<'b, F>>,

    /// domain X, sized for the public input
    domain_x: EvaluationDomain<F>,

    /// domain H, sized for constraints
    domain_h: EvaluationDomain<F>,

    /// domain K, sized for matrix nonzero elements
    domain_k: EvaluationDomain<F>,

    #[doc(hidden)]
    _field: PhantomData<C>,
    _cs: PhantomData<C>,
}

impl<'a, 'b, F: PrimeField, C> ProverState<'a, 'b, F, C> {
    /// Get the public input.
    pub fn public_input(&self) -> Vec<F> {
        ProverConstraintSystem::unformat_public_input(&self.formatted_input_assignment)
    }
}

/// Each prover message that is not a list of oracles is a list of field elements.
#[repr(transparent)]
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct ProverMsg<F: Field> {
    /// The field elements that make up the message
    pub field_elements: Vec<F>,
}

impl<F: Field> ToBytes for ProverMsg<F> {
    fn write<W: Write>(&self, mut w: W) -> io::Result<()> {
        CanonicalSerialize::serialize(self, &mut w).map_err(|_| error("Could not serialize ProverMsg"))
    }
}

/// The first set of prover oracles.
pub struct ProverFirstOracles<'b, F: Field> {
    /// The LDE of `w`.
    pub w: LabeledPolynomial<'b, F>,
    /// The LDE of `Az`.
    pub z_a: LabeledPolynomial<'b, F>,
    /// The LDE of `Bz`.
    pub z_b: LabeledPolynomial<'b, F>,
    /// The sum-check hiding polynomial.
    pub mask_poly: LabeledPolynomial<'b, F>,
}

impl<'b, F: Field> ProverFirstOracles<'b, F> {
    /// Iterate over the polynomials output by the prover in the first round.
    pub fn iter(&self) -> impl Iterator<Item = &LabeledPolynomial<'b, F>> {
        vec![&self.w, &self.z_a, &self.z_b, &self.mask_poly].into_iter()
    }
}

/// The second set of prover oracles.
pub struct ProverSecondOracles<'b, F: Field> {
    /// The polynomial `t` that is produced in the first round.
    pub t: LabeledPolynomial<'b, F>,
    /// The polynomial `g` resulting from the first sumcheck.
    pub g_1: LabeledPolynomial<'b, F>,
    /// The polynomial `h` resulting from the first sumcheck.
    pub h_1: LabeledPolynomial<'b, F>,
}

impl<'b, F: Field> ProverSecondOracles<'b, F> {
    /// Iterate over the polynomials output by the prover in the second round.
    pub fn iter(&self) -> impl Iterator<Item = &LabeledPolynomial<'b, F>> {
        vec![&self.t, &self.g_1, &self.h_1].into_iter()
    }
}

/// The third set of prover oracles.
pub struct ProverThirdOracles<'b, F: Field> {
    /// The polynomial `g` resulting from the second sumcheck.
    pub g_2: LabeledPolynomial<'b, F>,
    /// The polynomial `h` resulting from the second sumcheck.
    pub h_2: LabeledPolynomial<'b, F>,
}

impl<'b, F: Field> ProverThirdOracles<'b, F> {
    /// Iterate over the polynomials output by the prover in the third round.
    pub fn iter(&self) -> impl Iterator<Item = &LabeledPolynomial<'b, F>> {
        vec![&self.g_2, &self.h_2].into_iter()
    }
}

impl<F: PrimeField> AHPForR1CS<F> {
    /// Initialize the AHP prover.
    pub fn prover_init<'a, 'b, C: ConstraintSynthesizer<F>>(
        index: &'a Index<F, C>,
        c: C,
    ) -> Result<ProverState<'a, 'b, F, C>, Error> {
        let init_time = start_timer!(|| "AHP::Prover::Init");

        let constraint_time = start_timer!(|| "Generating constraints and witnesses");
        let mut pcs = ProverConstraintSystem::new();
        c.generate_constraints(&mut pcs)?;
        end_timer!(constraint_time);

        let padding_time = start_timer!(|| "Padding matrices to make them square");
        pcs.make_matrices_square();
        end_timer!(padding_time);

        let num_non_zero = index.index_info.num_non_zero;

        let ProverConstraintSystem {
            input_assignment: formatted_input_assignment,
            witness_assignment,
            num_constraints,
            ..
        } = pcs;

        let num_input_variables = formatted_input_assignment.len();
        let num_witness_variables = witness_assignment.len();
        if index.index_info.num_constraints != num_constraints
            || num_input_variables + num_witness_variables != index.index_info.num_variables
        {
            Err(Error::InstanceDoesNotMatchIndex)?;
        }

        if !Self::formatted_public_input_is_admissible(&formatted_input_assignment) {
            Err(Error::InvalidPublicInputLength)?
        }

        // Perform matrix multiplications
        let inner_prod_fn = |row: &[(F, usize)]| {
            let mut acc = F::zero();
            for &(ref coeff, i) in row {
                let tmp = if i < num_input_variables {
                    formatted_input_assignment[i]
                } else {
                    witness_assignment[i - num_input_variables]
                };

                acc += &(if coeff.is_one() { tmp } else { tmp * coeff });
            }
            acc
        };

        let eval_z_a_time = start_timer!(|| "Evaluating z_A");
        let z_a = index.a.iter().map(|row| inner_prod_fn(row)).collect();
        end_timer!(eval_z_a_time);

        let eval_z_b_time = start_timer!(|| "Evaluating z_B");
        let z_b = index.b.iter().map(|row| inner_prod_fn(row)).collect();
        end_timer!(eval_z_b_time);

        let zk_bound = 1; // One query is sufficient for our desired soundness

        let domain_h = EvaluationDomain::new(num_constraints).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;

        let domain_k = EvaluationDomain::new(num_non_zero).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;

        let domain_x = EvaluationDomain::new(num_input_variables).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;

        end_timer!(init_time);

        Ok(ProverState {
            formatted_input_assignment,
            witness_assignment,
            z_a: Some(z_a),
            z_b: Some(z_b),
            w_poly: None,
            mz_polys: None,
            zk_bound,
            index,
            verifier_first_msg: None,
            mask_poly: None,
            domain_h,
            domain_k,
            domain_x,
            _field: PhantomData,
            _cs: PhantomData,
        })
    }

    /// Output the first round message and the next state.
    pub fn prover_first_round<'a, 'b, R: RngCore, C: ConstraintSynthesizer<F>>(
        mut state: ProverState<'a, 'b, F, C>,
        rng: &mut R,
    ) -> Result<(ProverMsg<F>, ProverFirstOracles<'b, F>, ProverState<'a, 'b, F, C>), Error> {
        let round_time = start_timer!(|| "AHP::Prover::FirstRound");
        let domain_h = state.domain_h;
        let zk_bound = state.zk_bound;

        let v_H = domain_h.vanishing_polynomial().into();

        let x_time = start_timer!(|| "Computing x polynomial and evals");
        let domain_x = state.domain_x;
        let x_poly =
            EvaluationsOnDomain::from_vec_and_domain(state.formatted_input_assignment.clone(), domain_x).interpolate();
        let x_evals = domain_h.fft(&x_poly);
        end_timer!(x_time);

        let ratio = domain_h.size() / domain_x.size();

        let mut w_extended = state.witness_assignment.clone();
        w_extended.extend(vec![
            F::zero();
            domain_h.size() - domain_x.size() - state.witness_assignment.len()
        ]);

        let w_poly_time = start_timer!(|| "Computing w polynomial");
        let w_poly_evals = cfg_into_iter!(0..domain_h.size())
            .map(|k| {
                if k % ratio == 0 {
                    F::zero()
                } else {
                    w_extended[k - (k / ratio) - 1] - &x_evals[k]
                }
            })
            .collect();

        let w_poly = &EvaluationsOnDomain::from_vec_and_domain(w_poly_evals, domain_h).interpolate()
            + &(&Polynomial::from_coefficients_slice(&[F::rand(rng)]) * &v_H);
        let (w_poly, remainder) = w_poly.divide_by_vanishing_poly(domain_x).unwrap();
        assert!(remainder.is_zero());
        end_timer!(w_poly_time);

        let z_a_poly_time = start_timer!(|| "Computing z_A polynomial");
        let z_a = state.z_a.clone().unwrap();
        let z_a_poly = &EvaluationsOnDomain::from_vec_and_domain(z_a, domain_h).interpolate()
            + &(&Polynomial::from_coefficients_slice(&[F::rand(rng)]) * &v_H);
        end_timer!(z_a_poly_time);

        let z_b_poly_time = start_timer!(|| "Computing z_B polynomial");
        let z_b = state.z_b.clone().unwrap();
        let z_b_poly = &EvaluationsOnDomain::from_vec_and_domain(z_b, domain_h).interpolate()
            + &(&Polynomial::from_coefficients_slice(&[F::rand(rng)]) * &v_H);
        end_timer!(z_b_poly_time);

        let mask_poly_time = start_timer!(|| "Computing mask polynomial");
        let mask_poly_degree = 3 * domain_h.size() + 2 * zk_bound - 3;
        let mut mask_poly = Polynomial::rand(mask_poly_degree, rng);
        let scaled_sigma_1 = (mask_poly.divide_by_vanishing_poly(domain_h).unwrap().1)[0];
        mask_poly[0] -= &scaled_sigma_1;
        end_timer!(mask_poly_time);

        let msg = ProverMsg { field_elements: vec![] };

        assert!(w_poly.degree() <= domain_h.size() - domain_x.size() + zk_bound - 1);
        assert!(z_a_poly.degree() <= domain_h.size() + zk_bound - 1);
        assert!(z_b_poly.degree() <= domain_h.size() + zk_bound - 1);
        assert!(mask_poly.degree() <= 3 * domain_h.size() + 2 * zk_bound - 3);

        let w = LabeledPolynomial::new_owned("w".to_string(), w_poly, None, Some(1));
        let z_a = LabeledPolynomial::new_owned("z_a".to_string(), z_a_poly, None, Some(1));
        let z_b = LabeledPolynomial::new_owned("z_b".to_string(), z_b_poly, None, Some(1));
        let mask_poly = LabeledPolynomial::new_owned("mask_poly".to_string(), mask_poly.clone(), None, None);

        let oracles = ProverFirstOracles {
            w: w.clone(),
            z_a: z_a.clone(),
            z_b: z_b.clone(),
            mask_poly: mask_poly.clone(),
        };

        state.w_poly = Some(w);
        state.mz_polys = Some((z_a, z_b));
        state.mask_poly = Some(mask_poly);
        end_timer!(round_time);

        Ok((msg, oracles, state))
    }

    fn calculate_t<'a>(
        matrices: impl Iterator<Item = &'a Matrix<F>>,
        matrix_randomizers: &[F],
        input_domain: EvaluationDomain<F>,
        domain_h: EvaluationDomain<F>,
        r_alpha_x_on_h: Vec<F>,
    ) -> Polynomial<F> {
        let mut t_evals_on_h = vec![F::zero(); domain_h.size()];
        for (matrix, eta) in matrices.zip(matrix_randomizers) {
            for (r, row) in matrix.iter().enumerate() {
                for (coeff, c) in row.iter() {
                    let index = domain_h.reindex_by_subdomain(input_domain, *c);
                    t_evals_on_h[index] += &(*eta * coeff * &r_alpha_x_on_h[r]);
                }
            }
        }
        EvaluationsOnDomain::from_vec_and_domain(t_evals_on_h, domain_h).interpolate()
    }

    /// Output the number of oracles sent by the prover in the first round.
    pub fn prover_num_first_round_oracles() -> usize {
        4
    }

    /// Output the degree bounds of oracles in the first round.
    pub fn prover_first_round_degree_bounds<C: ConstraintSynthesizer<F>>(
        _info: &IndexInfo<F, C>,
    ) -> impl Iterator<Item = Option<usize>> {
        vec![None; 4].into_iter()
    }

    /// Output the second round message and the next state.
    pub fn prover_second_round<'a, 'b, R: RngCore, C: ConstraintSynthesizer<F>>(
        ver_message: &VerifierFirstMsg<F>,
        mut state: ProverState<'a, 'b, F, C>,
        _r: &mut R,
    ) -> (ProverMsg<F>, ProverSecondOracles<'b, F>, ProverState<'a, 'b, F, C>) {
        let round_time = start_timer!(|| "AHP::Prover::SecondRound");

        let domain_h = state.domain_h;
        let zk_bound = state.zk_bound;

        let mask_poly = state
            .mask_poly
            .as_ref()
            .expect("ProverState should include mask_poly when prover_second_round is called");

        let VerifierFirstMsg {
            alpha,
            eta_a,
            eta_b,
            eta_c,
        } = *ver_message;

        let summed_z_m_poly_time = start_timer!(|| "Compute z_m poly");
        let (z_a_poly, z_b_poly) = state.mz_polys.as_ref().unwrap();
        let z_c_poly = z_a_poly.polynomial() * z_b_poly.polynomial();

        let mut summed_z_m_coeffs = z_c_poly.coeffs;
        // Note: Can't combine these two loops, because z_c_poly has 2x the degree
        // of z_a_poly and z_b_poly, so the second loop gets truncated due to
        // the `zip`s.
        cfg_iter_mut!(summed_z_m_coeffs).for_each(|c| *c *= &eta_c);
        cfg_iter_mut!(summed_z_m_coeffs)
            .zip(&z_a_poly.polynomial().coeffs)
            .zip(&z_b_poly.polynomial().coeffs)
            .for_each(|((c, a), b)| *c += &(eta_a * a + &(eta_b * b)));

        let summed_z_m = Polynomial::from_coefficients_vec(summed_z_m_coeffs);
        end_timer!(summed_z_m_poly_time);

        let r_alpha_x_evals_time = start_timer!(|| "Compute r_alpha_x evals");
        let r_alpha_x_evals = domain_h.batch_eval_unnormalized_bivariate_lagrange_poly_with_diff_inputs(alpha);
        end_timer!(r_alpha_x_evals_time);

        let r_alpha_poly_time = start_timer!(|| "Compute r_alpha_x poly");
        let r_alpha_poly = Polynomial::from_coefficients_vec(domain_h.ifft(&r_alpha_x_evals));
        end_timer!(r_alpha_poly_time);

        let t_poly_time = start_timer!(|| "Compute t poly");
        let t_poly = Self::calculate_t(
            vec![&state.index.a, &state.index.b, &state.index.c].into_iter(),
            &[eta_a, eta_b, eta_c],
            state.domain_x,
            state.domain_h,
            r_alpha_x_evals,
        );
        end_timer!(t_poly_time);

        let z_poly_time = start_timer!(|| "Compute z poly");

        let domain_x = EvaluationDomain::new(state.formatted_input_assignment.len())
            .ok_or(SynthesisError::PolynomialDegreeTooLarge)
            .unwrap();
        let x_poly =
            EvaluationsOnDomain::from_vec_and_domain(state.formatted_input_assignment.clone(), domain_x).interpolate();
        let w_poly = state.w_poly.as_ref().unwrap();
        let mut z_poly = w_poly.polynomial().mul_by_vanishing_poly(domain_x);
        cfg_iter_mut!(z_poly.coeffs)
            .zip(&x_poly.coeffs)
            .for_each(|(z, x)| *z += x);
        assert!(z_poly.degree() <= domain_h.size() + zk_bound - 1);

        end_timer!(z_poly_time);

        let q_1_time = start_timer!(|| "Compute q_1 poly");

        let mul_domain_size = *[
            mask_poly.len(),
            r_alpha_poly.coeffs.len() + summed_z_m.coeffs.len(),
            t_poly.coeffs.len() + z_poly.len(),
        ]
        .iter()
        .max()
        .unwrap();
        let mul_domain =
            EvaluationDomain::new(mul_domain_size).expect("field is not smooth enough to construct domain");
        let mut r_alpha_evals = r_alpha_poly.evaluate_over_domain_by_ref(mul_domain);
        let summed_z_m_evals = summed_z_m.evaluate_over_domain_by_ref(mul_domain);
        let z_poly_evals = z_poly.evaluate_over_domain_by_ref(mul_domain);
        let t_poly_m_evals = t_poly.evaluate_over_domain_by_ref(mul_domain);

        cfg_iter_mut!(r_alpha_evals.evals)
            .zip(&summed_z_m_evals.evals)
            .zip(&z_poly_evals.evals)
            .zip(&t_poly_m_evals.evals)
            .for_each(|(((a, b), &c), d)| {
                *a *= &b;
                *a -= &(c * &d);
            });
        let rhs = r_alpha_evals.interpolate();
        let q_1 = mask_poly.polynomial() + &rhs;
        end_timer!(q_1_time);

        let sumcheck_time = start_timer!(|| "Compute sumcheck h and g polys");
        let (h_1, x_g_1) = q_1.divide_by_vanishing_poly(domain_h).unwrap();
        let g_1 = Polynomial::from_coefficients_slice(&x_g_1.coeffs[1..]);
        end_timer!(sumcheck_time);

        let msg = ProverMsg {
            field_elements: Vec::new(),
        };

        assert!(g_1.degree() <= domain_h.size() - 2);
        assert!(h_1.degree() <= 2 * domain_h.size() + 2 * zk_bound - 2);

        let oracles = ProverSecondOracles {
            t: LabeledPolynomial::new_owned("t".into(), t_poly, None, None),
            g_1: LabeledPolynomial::new_owned("g_1".into(), g_1, Some(domain_h.size() - 2), Some(1)),
            h_1: LabeledPolynomial::new_owned("h_1".into(), h_1, None, None),
        };

        state.w_poly = None;
        state.verifier_first_msg = Some(*ver_message);
        end_timer!(round_time);

        (msg, oracles, state)
    }

    /// Output the number of oracles sent by the prover in the second round.
    pub fn prover_num_second_round_oracles() -> usize {
        3
    }

    /// Output the degree bounds of oracles in the second round.
    pub fn prover_second_round_degree_bounds<C: ConstraintSynthesizer<F>>(
        info: &IndexInfo<F, C>,
    ) -> impl Iterator<Item = Option<usize>> {
        let h_domain_size = EvaluationDomain::<F>::compute_size_of_domain(info.num_constraints).unwrap();

        vec![None, Some(h_domain_size - 2), None].into_iter()
    }

    /// Output the third round message and the next state.
    pub fn prover_third_round<'a, 'b, R: RngCore, C: ConstraintSynthesizer<F>>(
        ver_message: &VerifierSecondMsg<F>,
        prover_state: ProverState<'a, 'b, F, C>,
        _r: &mut R,
    ) -> Result<(ProverMsg<F>, ProverThirdOracles<'b, F>), Error> {
        let round_time = start_timer!(|| "AHP::Prover::ThirdRound");

        let ProverState {
            index,
            verifier_first_msg,
            domain_h,
            domain_k,
            ..
        } = prover_state;

        let VerifierFirstMsg {
            eta_a,
            eta_b,
            eta_c,
            alpha,
        } = verifier_first_msg
            .expect("ProverState should include verifier_first_msg when prover_third_round is called");

        let beta = ver_message.beta;

        let v_H_at_alpha = domain_h.evaluate_vanishing_polynomial(alpha);
        let v_H_at_beta = domain_h.evaluate_vanishing_polynomial(beta);

        let (a_star, b_star, c_star) = (&index.a_star_arith, &index.b_star_arith, &index.c_star_arith);

        let f_evals_time = start_timer!(|| "Computing f evals on K");
        let mut f_vals_on_K = Vec::with_capacity(domain_k.size());
        let mut inverses_a = Vec::with_capacity(domain_k.size());
        let mut inverses_b = Vec::with_capacity(domain_k.size());
        let mut inverses_c = Vec::with_capacity(domain_k.size());

        for i in 0..domain_k.size() {
            inverses_a.push((beta - &a_star.evals_on_K.row[i]) * &(alpha - &a_star.evals_on_K.col[i]));
            inverses_b.push((beta - &b_star.evals_on_K.row[i]) * &(alpha - &b_star.evals_on_K.col[i]));
            inverses_c.push((beta - &c_star.evals_on_K.row[i]) * &(alpha - &c_star.evals_on_K.col[i]));
        }
        batch_inversion(&mut inverses_a);
        batch_inversion(&mut inverses_b);
        batch_inversion(&mut inverses_c);

        for i in 0..domain_k.size() {
            let t = eta_a * &a_star.evals_on_K.val[i] * &inverses_a[i]
                + &(eta_b * &b_star.evals_on_K.val[i] * &inverses_b[i])
                + &(eta_c * &c_star.evals_on_K.val[i] * &inverses_c[i]);
            let f_at_kappa = v_H_at_beta * &v_H_at_alpha * &t;
            f_vals_on_K.push(f_at_kappa);
        }
        end_timer!(f_evals_time);

        let f_poly_time = start_timer!(|| "Computing f poly");
        let f = EvaluationsOnDomain::from_vec_and_domain(f_vals_on_K, domain_k).interpolate();
        end_timer!(f_poly_time);

        let g_2 = Polynomial::from_coefficients_slice(&f.coeffs[1..]);

        let domain_b =
            EvaluationDomain::<F>::new(3 * domain_k.size() - 3).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;

        let denom_eval_time = start_timer!(|| "Computing denominator evals on B");
        let a_denom: Vec<_> = cfg_iter!(a_star.evals_on_B.row.evals)
            .zip(&a_star.evals_on_B.col.evals)
            .zip(&a_star.row_col_evals_on_B.evals)
            .map(|((&r, c), r_c)| beta * &alpha - &(r * &alpha) - &(beta * &c) + &r_c)
            .collect();

        let b_denom: Vec<_> = cfg_iter!(b_star.evals_on_B.row.evals)
            .zip(&b_star.evals_on_B.col.evals)
            .zip(&b_star.row_col_evals_on_B.evals)
            .map(|((&r, c), r_c)| beta * &alpha - &(r * &alpha) - &(beta * &c) + &r_c)
            .collect();

        let c_denom: Vec<_> = cfg_iter!(c_star.evals_on_B.row.evals)
            .zip(&c_star.evals_on_B.col.evals)
            .zip(&c_star.row_col_evals_on_B.evals)
            .map(|((&r, c), r_c)| beta * &alpha - &(r * &alpha) - &(beta * &c) + &r_c)
            .collect();
        end_timer!(denom_eval_time);

        let a_evals_time = start_timer!(|| "Computing a evals on B");
        let a_poly_on_B = cfg_into_iter!(0..domain_b.size())
            .map(|i| {
                let t = eta_a * &a_star.evals_on_B.val.evals[i] * &b_denom[i] * &c_denom[i]
                    + &(eta_b * &b_star.evals_on_B.val.evals[i] * &a_denom[i] * &c_denom[i])
                    + &(eta_c * &c_star.evals_on_B.val.evals[i] * &a_denom[i] * &b_denom[i]);
                v_H_at_beta * &v_H_at_alpha * &t
            })
            .collect();
        end_timer!(a_evals_time);

        let a_poly_time = start_timer!(|| "Computing a poly");
        let a_poly = EvaluationsOnDomain::from_vec_and_domain(a_poly_on_B, domain_b).interpolate();
        end_timer!(a_poly_time);

        let b_evals_time = start_timer!(|| "Computing b evals on B");
        let b_poly_on_B = cfg_into_iter!(0..domain_b.size())
            .map(|i| a_denom[i] * &b_denom[i] * &c_denom[i])
            .collect();
        end_timer!(b_evals_time);

        let b_poly_time = start_timer!(|| "Computing b poly");
        let b_poly = EvaluationsOnDomain::from_vec_and_domain(b_poly_on_B, domain_b).interpolate();
        end_timer!(b_poly_time);

        let h_2_poly_time = start_timer!(|| "Computing sumcheck h poly");
        let h_2 = (&a_poly - &(&b_poly * &f))
            .divide_by_vanishing_poly(domain_k)
            .unwrap()
            .0;
        end_timer!(h_2_poly_time);

        let msg = ProverMsg { field_elements: vec![] };

        assert!(g_2.degree() <= domain_k.size() - 2);
        let oracles = ProverThirdOracles {
            g_2: LabeledPolynomial::new_owned("g_2".to_string(), g_2, Some(domain_k.size() - 2), None),
            h_2: LabeledPolynomial::new_owned("h_2".to_string(), h_2, None, None),
        };
        end_timer!(round_time);

        Ok((msg, oracles))
    }

    /// Output the number of oracles sent by the prover in the third round.
    pub fn prover_num_third_round_oracles() -> usize {
        3
    }

    /// Output the degree bounds of oracles in the third round.
    pub fn prover_third_round_degree_bounds<C: ConstraintSynthesizer<F>>(
        info: &IndexInfo<F, C>,
    ) -> impl Iterator<Item = Option<usize>> {
        let num_non_zero = info.num_non_zero;
        let k_size = EvaluationDomain::<F>::compute_size_of_domain(num_non_zero).unwrap();

        vec![Some(k_size - 2), None].into_iter()
    }
}
