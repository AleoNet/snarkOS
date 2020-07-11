use super::{generator::KeypairAssembly, prover::ProvingAssignment, Vec};
use crate::{cfg_iter, cfg_iter_mut, fft::EvaluationDomain};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{PairingEngine, Zero},
    gadgets::r1cs::{ConstraintSystem, Index},
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

fn evaluate_constraint<E: PairingEngine>(terms: &[(E::Fr, Index)], assignment: &[E::Fr], num_input: usize) -> E::Fr {
    let mut acc = E::Fr::zero();
    for &(coeff, index) in terms {
        let val = match index {
            Index::Input(i) => assignment[i],
            Index::Aux(i) => assignment[num_input + i],
        };
        acc += &(val * &coeff);
    }
    acc
}

pub(crate) struct R1CStoQAP;

impl R1CStoQAP {
    #[inline]
    pub(crate) fn instance_map_with_evaluation<E: PairingEngine>(
        assembly: &KeypairAssembly<E>,
        t: &E::Fr,
    ) -> Result<(Vec<E::Fr>, Vec<E::Fr>, Vec<E::Fr>, E::Fr, usize, usize), SynthesisError> {
        let domain_size = assembly.num_constraints + (assembly.num_inputs - 1) + 1;
        let domain = EvaluationDomain::<E::Fr>::new(domain_size).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;
        let domain_size = domain.size();

        let zt = domain.evaluate_vanishing_polynomial(*t);

        // Evaluate all Lagrange polynomials
        let coefficients_time = start_timer!(|| "Evaluate Lagrange coefficients");
        let u = domain.evaluate_all_lagrange_coefficients(*t);
        end_timer!(coefficients_time);

        let qap_num_variables = (assembly.num_inputs - 1) + assembly.num_aux;

        let mut a = vec![E::Fr::zero(); qap_num_variables + 1];
        let mut b = vec![E::Fr::zero(); qap_num_variables + 1];
        let mut c = vec![E::Fr::zero(); qap_num_variables + 1];

        for i in 0..assembly.num_inputs {
            a[i] = u[assembly.num_constraints + i];
        }

        for i in 0..assembly.num_constraints {
            for &(ref coeff, index) in assembly.at[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                a[index] += &(u[i] * coeff);
            }
            for &(ref coeff, index) in assembly.bt[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                b[index] += &(u[i] * coeff);
            }
            for &(ref coeff, index) in assembly.ct[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                c[index] += &(u[i] * coeff);
            }
        }

        Ok((a, b, c, zt, qap_num_variables, domain_size))
    }

    #[inline]
    pub(crate) fn witness_map<E: PairingEngine>(prover: &ProvingAssignment<E>) -> Result<Vec<E::Fr>, SynthesisError> {
        let zero = E::Fr::zero();
        let num_inputs = prover.input_assignment.len();
        let num_constraints = prover.num_constraints();

        let full_input_assignment = [&prover.input_assignment[..], &prover.aux_assignment[..]].concat();

        let domain = EvaluationDomain::<E::Fr>::new(num_constraints + num_inputs)
            .ok_or(SynthesisError::PolynomialDegreeTooLarge)?;
        let domain_size = domain.size();

        let mut a = vec![zero; domain_size];
        let mut b = vec![zero; domain_size];

        cfg_iter_mut!(a[..num_constraints])
            .zip(cfg_iter_mut!(b[..num_constraints]))
            .zip(cfg_iter!(&prover.at))
            .zip(cfg_iter!(&prover.bt))
            .for_each(|(((a, b), at_i), bt_i)| {
                *a = evaluate_constraint::<E>(&at_i, &full_input_assignment, num_inputs);
                *b = evaluate_constraint::<E>(&bt_i, &full_input_assignment, num_inputs);
            });

        for i in 0..num_inputs {
            a[num_constraints + i] = full_input_assignment[i];
        }

        domain.ifft_in_place(&mut a);
        domain.ifft_in_place(&mut b);

        domain.coset_fft_in_place(&mut a);
        domain.coset_fft_in_place(&mut b);

        let mut ab = domain.mul_polynomials_in_evaluation_domain(&a, &b);
        drop(a);
        drop(b);

        let mut c = vec![zero; domain_size];
        cfg_iter_mut!(c[..prover.num_constraints()])
            .enumerate()
            .for_each(|(i, c)| {
                *c = evaluate_constraint::<E>(&prover.ct[i], &full_input_assignment, num_inputs);
            });

        domain.ifft_in_place(&mut c);
        domain.coset_fft_in_place(&mut c);

        cfg_iter_mut!(ab).zip(c).for_each(|(ab_i, c_i)| *ab_i -= &c_i);

        domain.divide_by_vanishing_poly_on_coset_in_place(&mut ab);
        domain.coset_ifft_in_place(&mut ab);

        Ok(ab)
    }
}
