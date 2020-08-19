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

use super::{generator::KeypairAssembly, prover::ProvingAssignment};
use crate::fft::EvaluationDomain;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{Field, One, PairingEngine, Zero},
    gadgets::r1cs::Index,
};

use std::ops::{AddAssign, SubAssign};

pub(crate) struct R1CStoSAP;

impl R1CStoSAP {
    #[inline]
    pub(crate) fn instance_map_with_evaluation<E: PairingEngine>(
        assembly: &KeypairAssembly<E>,
        t: &E::Fr,
    ) -> Result<(Vec<E::Fr>, Vec<E::Fr>, E::Fr, usize, usize), SynthesisError> {
        let domain_size = 2 * assembly.num_constraints + 2 * (assembly.num_inputs - 1) + 1;
        let domain = EvaluationDomain::<E::Fr>::new(domain_size).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;
        let domain_size = domain.size();

        let zt = domain.evaluate_vanishing_polynomial(*t);

        // Evaluate all Lagrange polynomials
        let coefficients_time = start_timer!(|| "Evaluate Lagrange coefficients");
        let u = domain.evaluate_all_lagrange_coefficients(*t);
        end_timer!(coefficients_time);

        let sap_num_variables = 2 * (assembly.num_inputs - 1) + assembly.num_aux + assembly.num_constraints;
        let extra_var_offset = (assembly.num_inputs - 1) + assembly.num_aux + 1;
        let extra_constr_offset = 2 * assembly.num_constraints;
        let extra_var_offset2 = (assembly.num_inputs - 1) + assembly.num_aux + assembly.num_constraints;

        let mut a = vec![E::Fr::zero(); sap_num_variables + 1];
        let mut c = vec![E::Fr::zero(); sap_num_variables + 1];

        for i in 0..assembly.num_constraints {
            let u_2i = u[2 * i];
            let u_2i_plus_1 = u[2 * i + 1];
            let u_add = u_2i + &u_2i_plus_1;
            let u_sub = u_2i - &u_2i_plus_1;

            for &(ref coeff, index) in assembly.at[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                a[index] += &(u_add * coeff);
            }

            for &(ref coeff, index) in assembly.bt[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                a[index] += &(u_sub * coeff);
            }

            for &(ref coeff, index) in assembly.ct[i].iter() {
                let index = match index {
                    Index::Input(i) => i,
                    Index::Aux(i) => assembly.num_inputs + i,
                };

                c[index] += &((u_2i * coeff).double().double());
            }
            c[extra_var_offset + i].add_assign(&u_add);
        }

        a[0].add_assign(&u[extra_constr_offset]);
        c[0].add_assign(&u[extra_constr_offset]);

        for i in 1..assembly.num_inputs {
            // First extra constraint

            a[i].add_assign(&u[extra_constr_offset + 2 * i - 1]);
            a[0].add_assign(&u[extra_constr_offset + 2 * i - 1]);

            let t_four = u[extra_constr_offset + 2 * i - 1].double().double();

            c[i].add_assign(&t_four);
            c[extra_var_offset2 + i].add_assign(&u[extra_constr_offset + 2 * i - 1]);

            // Second extra constraint

            a[i].add_assign(&u[extra_constr_offset + 2 * i]);
            a[0].sub_assign(&u[extra_constr_offset + 2 * i]);
            c[extra_var_offset2 + i].add_assign(&u[extra_constr_offset + 2 * i]);
        }

        Ok((a, c, zt, sap_num_variables, domain_size))
    }

    #[inline]
    pub(crate) fn witness_map<E: PairingEngine>(
        prover: &ProvingAssignment<E>,
        d1: &E::Fr,
        d2: &E::Fr,
    ) -> Result<(Vec<E::Fr>, Vec<E::Fr>, usize), SynthesisError> {
        #[inline]
        fn evaluate_constraint<E: PairingEngine>(
            terms: &[(E::Fr, Index)],
            assignment: &[E::Fr],
            num_input: usize,
        ) -> E::Fr {
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

        let zero = E::Fr::zero();
        let one = E::Fr::one();

        let mut full_input_assignment = prover.input_assignment.clone();
        full_input_assignment.extend(prover.aux_assignment.clone());

        let temp = cfg_iter!(prover.at)
            .zip(&prover.bt)
            .map(|(a_i, b_i)| {
                let mut extra_var: E::Fr = evaluate_constraint::<E>(&a_i, &full_input_assignment, prover.num_inputs);
                extra_var.sub_assign(&evaluate_constraint::<E>(
                    &b_i,
                    &full_input_assignment,
                    prover.num_inputs,
                ));
                extra_var.square_in_place();
                extra_var
            })
            .collect::<Vec<_>>();
        full_input_assignment.extend(temp);

        for i in 1..prover.num_inputs {
            let mut extra_var = full_input_assignment[i];
            extra_var.sub_assign(&one);
            extra_var.square_in_place();
            full_input_assignment.push(extra_var);
        }

        let domain = EvaluationDomain::<E::Fr>::new(2 * prover.num_constraints + 2 * (prover.num_inputs - 1) + 1)
            .ok_or(SynthesisError::PolynomialDegreeTooLarge)?;
        let domain_size = domain.size();

        let extra_constr_offset = 2 * prover.num_constraints;
        let extra_var_offset = prover.num_inputs + prover.num_aux;
        let extra_var_offset2 = prover.num_inputs + prover.num_aux + prover.num_constraints - 1;

        let mut a = vec![zero; domain_size];
        cfg_chunks_mut!(a[..2 * prover.num_constraints], 2)
            .zip(&prover.at)
            .zip(&prover.bt)
            .for_each(|((chunk, at_i), bt_i)| {
                chunk[0] = evaluate_constraint::<E>(&at_i, &full_input_assignment, prover.num_inputs);
                chunk[0].add_assign(&evaluate_constraint::<E>(
                    &bt_i,
                    &full_input_assignment,
                    prover.num_inputs,
                ));

                chunk[1] = evaluate_constraint::<E>(&at_i, &full_input_assignment, prover.num_inputs);
                chunk[1].sub_assign(&evaluate_constraint::<E>(
                    &bt_i,
                    &full_input_assignment,
                    prover.num_inputs,
                ));
            });
        a[extra_constr_offset] = one;
        for i in 1..prover.num_inputs {
            a[extra_constr_offset + 2 * i - 1] = full_input_assignment[i] + &one;
            a[extra_constr_offset + 2 * i] = full_input_assignment[i] - &one;
        }

        domain.ifft_in_place(&mut a);

        let d1_double = d1.double();
        let mut h: Vec<E::Fr> = vec![d1_double; domain_size];
        cfg_iter_mut!(h).zip(&a).for_each(|(h_i, a_i)| *h_i *= a_i);
        h[0].sub_assign(&d2);
        let d1d1 = d1.square();
        h[0].sub_assign(&d1d1);
        h.push(d1d1);

        domain.coset_fft_in_place(&mut a);

        let mut aa = domain.mul_polynomials_in_evaluation_domain(&a, &a);
        drop(a);

        let mut c = vec![zero; domain_size];
        cfg_chunks_mut!(c[..2 * prover.num_constraints], 2)
            .enumerate()
            .for_each(|(i, chunk)| {
                let mut tmp: E::Fr = evaluate_constraint::<E>(&prover.ct[i], &full_input_assignment, prover.num_inputs);
                tmp.double_in_place();
                tmp.double_in_place();

                let assignment = full_input_assignment[extra_var_offset + i];
                chunk[0] = tmp + &assignment;
                chunk[1] = assignment;
            });
        c[extra_constr_offset] = one;
        for i in 1..prover.num_inputs {
            let mut tmp = full_input_assignment[i];
            tmp.double_in_place();
            tmp.double_in_place();

            let assignment = full_input_assignment[extra_var_offset2 + i];
            c[extra_constr_offset + 2 * i - 1] = tmp + &assignment;
            c[extra_constr_offset + 2 * i] = assignment;
        }

        domain.ifft_in_place(&mut c);
        domain.coset_fft_in_place(&mut c);

        cfg_iter_mut!(aa).zip(c).for_each(|(aa_i, c_i)| *aa_i -= &c_i);

        domain.divide_by_vanishing_poly_on_coset_in_place(&mut aa);
        domain.coset_ifft_in_place(&mut aa);

        cfg_iter_mut!(h[..domain_size - 1])
            .enumerate()
            .for_each(|(i, e)| e.add_assign(&aa[i]));

        Ok((full_input_assignment, h, domain_size))
    }
}
