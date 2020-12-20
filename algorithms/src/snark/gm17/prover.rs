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

use super::{r1cs_to_sap::R1CStoSAP, Parameters, Proof};
use crate::msm::VariableBaseMSM;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    curves::{AffineCurve, One, PairingEngine, PrimeField, ProjectiveCurve, Zero},
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem, Index, LinearCombination, Variable},
};
use snarkos_utilities::rand::UniformRand;

use rand::Rng;
use smallvec::SmallVec;
use std::ops::{AddAssign, MulAssign};

type CoeffVec<T> = SmallVec<[T; 2]>;

#[inline]
fn eval<E: PairingEngine>(
    lc: &LinearCombination<E::Fr>,
    constraints: &mut [CoeffVec<(E::Fr, Index)>],
    input_assignment: &[E::Fr],
    aux_assignment: &[E::Fr],
    this_constraint: usize,
) -> E::Fr {
    let mut acc = E::Fr::zero();

    for &(index, coeff) in lc.as_ref() {
        let mut tmp;

        match index.get_unchecked() {
            Index::Input(i) => {
                constraints[this_constraint].push((coeff, Index::Input(i)));
                tmp = input_assignment[i];
            }
            Index::Aux(i) => {
                constraints[this_constraint].push((coeff, Index::Aux(i)));
                tmp = aux_assignment[i];
            }
        }

        if coeff.is_one() {
            acc.add_assign(&tmp);
        } else {
            tmp.mul_assign(&coeff);
            acc.add_assign(&tmp);
        }
    }

    acc
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProvingAssignment<E: PairingEngine> {
    // Constraints
    pub(crate) at: Vec<CoeffVec<(E::Fr, Index)>>,
    pub(crate) bt: Vec<CoeffVec<(E::Fr, Index)>>,
    pub(crate) ct: Vec<CoeffVec<(E::Fr, Index)>>,

    // Evaluations of A and C polynomials
    pub(crate) a: Vec<E::Fr>,
    pub(crate) b: Vec<E::Fr>,
    pub(crate) c: Vec<E::Fr>,

    // Assignments of variables
    pub(crate) input_assignment: Vec<E::Fr>,
    pub(crate) aux_assignment: Vec<E::Fr>,
    pub(crate) num_inputs: usize,
    pub(crate) num_aux: usize,
    pub(crate) num_constraints: usize,
}

impl<E: PairingEngine> ProvingAssignment<E> {
    pub fn which_is_unsatisfied(&self) -> Option<usize> {
        for (i, ((a_i, b_i), c_i)) in (self.a.iter().zip(self.b.iter())).zip(self.c.iter()).enumerate() {
            if *a_i * b_i != *c_i {
                return Some(i);
            }
        }
        None
    }
}

impl<E: PairingEngine> ConstraintSystem<E::Fr> for ProvingAssignment<E> {
    type Root = Self;

    #[inline]
    fn alloc<F, A, AR>(&mut self, _: A, f: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<E::Fr, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.num_aux;
        self.num_aux += 1;

        self.aux_assignment.push(f()?);
        Ok(Variable::new_unchecked(Index::Aux(index)))
    }

    #[inline]
    fn alloc_input<F, A, AR>(&mut self, _: A, f: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<E::Fr, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        let index = self.num_inputs;
        self.num_inputs += 1;

        self.input_assignment.push(f()?);
        Ok(Variable::new_unchecked(Index::Input(index)))
    }

    #[inline]
    fn enforce<A, AR, LA, LB, LC>(&mut self, _: A, a: LA, b: LB, c: LC)
    where
        A: FnOnce() -> AR,
        AR: AsRef<str>,
        LA: FnOnce(LinearCombination<E::Fr>) -> LinearCombination<E::Fr>,
        LB: FnOnce(LinearCombination<E::Fr>) -> LinearCombination<E::Fr>,
        LC: FnOnce(LinearCombination<E::Fr>) -> LinearCombination<E::Fr>,
    {
        self.at.push(CoeffVec::new());
        self.bt.push(CoeffVec::new());
        self.ct.push(CoeffVec::new());

        self.a.push(eval::<E>(
            &a(LinearCombination::zero()),
            &mut self.at,
            &self.input_assignment,
            &self.aux_assignment,
            self.num_constraints,
        ));
        self.b.push(eval::<E>(
            &b(LinearCombination::zero()),
            &mut self.bt,
            &self.input_assignment,
            &self.aux_assignment,
            self.num_constraints,
        ));
        self.c.push(eval::<E>(
            &c(LinearCombination::zero()),
            &mut self.ct,
            &self.input_assignment,
            &self.aux_assignment,
            self.num_constraints,
        ));

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
        self.a.len()
    }
}

pub fn create_random_proof<E, C, R>(
    circuit: &C,
    params: &Parameters<E>,
    rng: &mut R,
) -> Result<Proof<E>, SynthesisError>
where
    E: PairingEngine,
    C: ConstraintSynthesizer<E::Fr>,
    R: Rng,
{
    let d1 = E::Fr::rand(rng);
    let d2 = E::Fr::rand(rng);
    let r = E::Fr::rand(rng);

    create_proof::<E, C>(circuit, params, d1, d2, r)
}

pub fn create_proof<E, C>(
    circuit: &C,
    params: &Parameters<E>,
    d1: E::Fr,
    d2: E::Fr,
    r: E::Fr,
) -> Result<Proof<E>, SynthesisError>
where
    E: PairingEngine,
    C: ConstraintSynthesizer<E::Fr>,
{
    let prover_time = start_timer!(|| "Prover");
    let mut prover = ProvingAssignment {
        at: vec![],
        bt: vec![],
        ct: vec![],
        a: vec![],
        b: vec![],
        c: vec![],
        input_assignment: vec![],
        aux_assignment: vec![],
        num_inputs: 0,
        num_aux: 0,
        num_constraints: 0,
    };

    // Allocate the "one" input variable
    prover.alloc_input(|| "", || Ok(E::Fr::one()))?;

    // Synthesize the circuit.
    let synthesis_time = start_timer!(|| "Constraint synthesis");
    circuit.generate_constraints(&mut prover)?;
    end_timer!(synthesis_time);

    let witness_map_time = start_timer!(|| "R1CS to SAP witness map");
    let (full_input_assignment, h, _) = R1CStoSAP::witness_map::<E>(&prover, &d1, &d2)?;
    end_timer!(witness_map_time);

    let input_assignment = full_input_assignment[1..prover.num_inputs]
        .iter()
        .map(|s| s.into_repr())
        .collect::<Vec<_>>();

    let aux_assignment = cfg_into_iter!(full_input_assignment[prover.num_inputs..])
        .map(|s| s.into_repr())
        .collect::<Vec<_>>();
    drop(full_input_assignment);

    let h_input = h[0..prover.num_inputs]
        .iter()
        .map(|s| s.into_repr())
        .collect::<Vec<_>>();
    let h_aux = cfg_into_iter!(h[prover.num_inputs..])
        .map(|s| s.into_repr())
        .collect::<Vec<_>>();
    drop(h);

    // Compute A
    let a_acc_time = start_timer!(|| "Compute A");
    let (a_inputs_source, a_aux_source) = params.get_a_query(prover.num_inputs)?;
    let a_inputs_acc = VariableBaseMSM::multi_scalar_mul(a_inputs_source, &input_assignment);
    let a_aux_acc = VariableBaseMSM::multi_scalar_mul(a_aux_source, &aux_assignment);

    let r_g = params.get_g_gamma_z()?.mul(r);
    let d1_g = params.get_g_gamma_z()?.mul(d1);

    let mut g_a = r_g;
    g_a.add_assign(&params.get_a_query_full()?[0].into_projective());
    g_a.add_assign(&d1_g);
    g_a.add_assign(&a_inputs_acc);
    g_a.add_assign(&a_aux_acc);
    end_timer!(a_acc_time);

    // Compute B
    let b_acc_time = start_timer!(|| "Compute B");

    let (b_inputs_source, b_aux_source) = params.get_b_query(prover.num_inputs)?;
    let b_inputs_acc = VariableBaseMSM::multi_scalar_mul(b_inputs_source, &input_assignment);
    let b_aux_acc = VariableBaseMSM::multi_scalar_mul(b_aux_source, &aux_assignment);

    let r_h = params.get_h_gamma_z()?.mul(r);
    let d1_h = params.get_h_gamma_z()?.mul(d1);

    let mut g_b = r_h;
    g_b.add_assign(&params.get_b_query_full()?[0].into_projective());
    g_b.add_assign(&d1_h);
    g_b.add_assign(&b_inputs_acc);
    g_b.add_assign(&b_aux_acc);
    end_timer!(b_acc_time);

    // Compute C
    let c_acc_time = start_timer!(|| "Compute C");
    let r_2 = r + &r;
    let r2 = r * &r;
    let d1_r_2 = d1 * &r_2;

    let c1_acc_time = start_timer!(|| "Compute C1");
    let (_, c1_aux_source) = params.get_c_query_1(0)?;
    let c1_acc = VariableBaseMSM::multi_scalar_mul(c1_aux_source, &aux_assignment);
    end_timer!(c1_acc_time);

    let c2_acc_time = start_timer!(|| "Compute C2");

    let (c2_inputs_source, c2_aux_source) = params.get_c_query_2(prover.num_inputs)?;
    let c2_inputs_acc = VariableBaseMSM::multi_scalar_mul(c2_inputs_source, &input_assignment);
    let c2_aux_acc = VariableBaseMSM::multi_scalar_mul(c2_aux_source, &aux_assignment);

    let c2_acc = c2_inputs_acc + &c2_aux_acc;
    end_timer!(c2_acc_time);

    // Compute G
    let g_acc_time = start_timer!(|| "Compute G");

    let (g_inputs_source, g_aux_source) = params.get_g_gamma2_z_t(prover.num_inputs)?;
    let g_inputs_acc = VariableBaseMSM::multi_scalar_mul(g_inputs_source, &h_input);
    let g_aux_acc = VariableBaseMSM::multi_scalar_mul(g_aux_source, &h_aux);

    let g_acc = g_inputs_acc + &g_aux_acc;
    end_timer!(g_acc_time);

    let r2_g_gamma2_z2 = params.get_g_gamma2_z2()?.mul(r2);
    let r_g_ab_gamma_z = params.get_g_ab_gamma_z()?.mul(r);
    let d1_g_ab_gamma_z = params.get_g_ab_gamma_z()?.mul(d1);
    let r_c0 = params.get_c_query_2_full()?[0].mul(r);
    let r2_d1_g_gamma2_z2 = params.get_g_gamma2_z2()?.mul(d1_r_2);
    let d2_g_gamma2_z_t0 = params.get_g_gamma2_z_t_full()?[0].mul(d2);
    let mut r_c2_exp = c2_acc;
    r_c2_exp.mul_assign(r);

    let mut g_c = c1_acc;
    g_c.add_assign(&r2_g_gamma2_z2);
    g_c.add_assign(&r_g_ab_gamma_z);
    g_c.add_assign(&d1_g_ab_gamma_z);
    g_c.add_assign(&r_c0);
    g_c.add_assign(&r2_d1_g_gamma2_z2);
    g_c.add_assign(&r_c2_exp);
    g_c.add_assign(&d2_g_gamma2_z_t0);
    g_c.add_assign(&g_acc);
    end_timer!(c_acc_time);

    end_timer!(prover_time);

    Ok(Proof {
        a: g_a.into_affine(),
        b: g_b.into_affine(),
        c: g_c.into_affine(),
    })
}
