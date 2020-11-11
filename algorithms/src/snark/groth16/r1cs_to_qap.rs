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

use super::{generator::KeypairAssembly, prover::ProvingAssignment, Vec};
use crate::{cfg_iter, cfg_iter_mut, fft::EvaluationDomain};
use snarkos_errors::gadgets::{SynthesisError, SynthesisResult};
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
    #[allow(clippy::many_single_char_names)]
    #[allow(clippy::type_complexity)]
    pub(crate) fn instance_map_with_evaluation<E: PairingEngine>(
        assembly: &KeypairAssembly<E>,
        t: &E::Fr,
    ) -> SynthesisResult<(Vec<E::Fr>, Vec<E::Fr>, Vec<E::Fr>, E::Fr, usize, usize)> {
        let domain_size = assembly.num_constraints() + (assembly.num_inputs - 1) + 1;
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
            a[i] = u[assembly.num_constraints() + i];
        }

        for (i, x) in u.iter().enumerate().take(assembly.num_constraints()) {
            for &(ref coeff, index) in assembly.at[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                a[index] += &(*x * coeff);
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
    pub(crate) fn witness_map<E: PairingEngine>(prover: &ProvingAssignment<E>) -> SynthesisResult<Vec<E::Fr>> {
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

        a[num_constraints..(num_inputs + num_constraints)].clone_from_slice(&full_input_assignment[..num_inputs]);

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
