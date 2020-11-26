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

use super::{push_constraints, r1cs_to_qap::R1CStoQAP, Parameters, VerifyingKey};
use crate::{cfg_into_iter, cfg_iter, fft::EvaluationDomain, msm::FixedBaseMSM};
use snarkos_errors::{gadgets::SynthesisError, serialization::SerializationError};
use snarkos_models::{
    curves::{Field, Group, One, PairingEngine, PrimeField, ProjectiveCurve, Zero},
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem, Index, LinearCombination, Variable},
};
use snarkos_profiler::{end_timer, start_timer};
use snarkos_utilities::{rand::UniformRand, serialize::*};

use rand::Rng;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Generates a random common reference string for
/// a circuit.
pub fn generate_random_parameters<E, C, R>(circuit: &C, rng: &mut R) -> Result<Parameters<E>, SynthesisError>
where
    E: PairingEngine,
    C: ConstraintSynthesizer<E::Fr>,
    R: Rng,
{
    let alpha = E::Fr::rand(rng);
    let beta = E::Fr::rand(rng);
    let gamma = E::Fr::rand(rng);
    let delta = E::Fr::rand(rng);

    generate_parameters::<E, C, R>(circuit, alpha, beta, gamma, delta, rng)
}

/// This is our assembly structure that we'll use to synthesize the
/// circuit into a QAP.
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct KeypairAssembly<E: PairingEngine> {
    pub num_inputs: usize,
    pub num_aux: usize,
    pub at: Vec<Vec<(E::Fr, Index)>>,
    pub bt: Vec<Vec<(E::Fr, Index)>>,
    pub ct: Vec<Vec<(E::Fr, Index)>>,
}

impl<E: PairingEngine> ConstraintSystem<E::Fr> for KeypairAssembly<E> {
    type Root = Self;

    #[inline]
    fn alloc<F, A, AR>(&mut self, _: A, _: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<E::Fr, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        // There is no assignment, so we don't invoke the
        // function for obtaining one.

        let index = self.num_aux;
        self.num_aux += 1;

        Ok(Variable::new_unchecked(Index::Aux(index)))
    }

    #[inline]
    fn alloc_input<F, A, AR>(&mut self, _: A, _: F) -> Result<Variable, SynthesisError>
    where
        F: FnOnce() -> Result<E::Fr, SynthesisError>,
        A: FnOnce() -> AR,
        AR: AsRef<str>,
    {
        // There is no assignment, so we don't invoke the
        // function for obtaining one.

        let index = self.num_inputs;
        self.num_inputs += 1;

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
        push_constraints(a(LinearCombination::zero()), &mut self.at);
        push_constraints(b(LinearCombination::zero()), &mut self.bt);
        push_constraints(c(LinearCombination::zero()), &mut self.ct);
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
        self.at.len()
    }
}

/// Create parameters for a circuit, given some toxic waste.
#[allow(clippy::many_single_char_names)]
pub fn generate_parameters<E, C, R>(
    circuit: &C,
    alpha: E::Fr,
    beta: E::Fr,
    gamma: E::Fr,
    delta: E::Fr,
    rng: &mut R,
) -> Result<Parameters<E>, SynthesisError>
where
    E: PairingEngine,
    C: ConstraintSynthesizer<E::Fr>,
    R: Rng,
{
    let mut assembly = KeypairAssembly {
        num_inputs: 0,
        num_aux: 0,
        at: vec![],
        bt: vec![],
        ct: vec![],
    };

    // Allocate the "one" input variable
    assembly.alloc_input(|| "", || Ok(E::Fr::one()))?;

    // Synthesize the circuit.
    let synthesis_time = start_timer!(|| "Constraint synthesis");
    circuit.generate_constraints(&mut assembly)?;
    end_timer!(synthesis_time);

    ///////////////////////////////////////////////////////////////////////////
    let domain_time = start_timer!(|| "Constructing evaluation domain");

    let domain_size = assembly.num_constraints() + (assembly.num_inputs - 1) + 1;
    let domain = EvaluationDomain::<E::Fr>::new(domain_size).ok_or(SynthesisError::PolynomialDegreeTooLarge)?;
    let t = domain.sample_element_outside_domain(rng);

    end_timer!(domain_time);
    ///////////////////////////////////////////////////////////////////////////

    let reduction_time = start_timer!(|| "R1CS to QAP Instance Map with Evaluation");
    let (a, b, c, zt, qap_num_variables, m_raw) = R1CStoQAP::instance_map_with_evaluation::<E>(&assembly, &t)?;
    end_timer!(reduction_time);

    // Compute query densities
    let non_zero_a: usize = cfg_into_iter!(0..qap_num_variables)
        .map(|i| (!a[i].is_zero()) as usize)
        .sum();

    let non_zero_b: usize = cfg_into_iter!(0..qap_num_variables)
        .map(|i| (!b[i].is_zero()) as usize)
        .sum();

    let scalar_bits = E::Fr::size_in_bits();

    let gamma_inverse = gamma.inverse().ok_or(SynthesisError::UnexpectedIdentity)?;
    let delta_inverse = delta.inverse().ok_or(SynthesisError::UnexpectedIdentity)?;

    let gamma_abc = cfg_iter!(a[0..assembly.num_inputs])
        .zip(&b[0..assembly.num_inputs])
        .zip(&c[0..assembly.num_inputs])
        .map(|((a, b), c)| (beta * a + &(alpha * b) + c) * &gamma_inverse)
        .collect::<Vec<_>>();

    let l = cfg_iter!(a)
        .zip(&b)
        .zip(&c)
        .map(|((a, b), c)| (beta * a + &(alpha * b) + c) * &delta_inverse)
        .collect::<Vec<_>>();

    let g1_generator = E::G1Projective::rand(rng);
    let g2_generator = E::G2Projective::rand(rng);

    // Compute G window table
    let g1_window_time = start_timer!(|| "Compute G1 window table");
    let g1_window = FixedBaseMSM::get_mul_window_size(non_zero_a + non_zero_b + qap_num_variables + m_raw + 1);
    let g1_table = FixedBaseMSM::get_window_table::<E::G1Projective>(scalar_bits, g1_window, g1_generator);
    end_timer!(g1_window_time);

    // Generate the R1CS proving key
    let proving_key_time = start_timer!(|| "Generate the R1CS proving key");

    let alpha_g1 = g1_generator.mul(&alpha);
    let beta_g1 = g1_generator.mul(&beta);
    let beta_g2 = g2_generator.mul(&beta);
    let delta_g1 = g1_generator.mul(&delta);
    let delta_g2 = g2_generator.mul(&delta);

    // Compute the A-query
    let a_time = start_timer!(|| "Calculate A");
    let mut a_query = FixedBaseMSM::multi_scalar_mul::<E::G1Projective>(scalar_bits, g1_window, &g1_table, &a);
    end_timer!(a_time);

    // Compute the B-query in G1
    let b_g1_time = start_timer!(|| "Calculate B G1");
    let mut b_g1_query = FixedBaseMSM::multi_scalar_mul::<E::G1Projective>(scalar_bits, g1_window, &g1_table, &b);
    end_timer!(b_g1_time);

    // Compute B window table
    let g2_time = start_timer!(|| "Compute G2 table");
    let g2_window = FixedBaseMSM::get_mul_window_size(non_zero_b);
    let g2_table = FixedBaseMSM::get_window_table::<E::G2Projective>(scalar_bits, g2_window, g2_generator);
    end_timer!(g2_time);

    // Compute the B-query in G2
    let b_g2_time = start_timer!(|| "Calculate B G2");
    let mut b_g2_query = FixedBaseMSM::multi_scalar_mul::<E::G2Projective>(scalar_bits, g2_window, &g2_table, &b);
    end_timer!(b_g2_time);

    // Compute the H-query
    let h_time = start_timer!(|| "Calculate H");
    let mut h_query = FixedBaseMSM::multi_scalar_mul::<E::G1Projective>(
        scalar_bits,
        g1_window,
        &g1_table,
        &cfg_into_iter!(0..m_raw - 1)
            .map(|i| zt * &delta_inverse * &t.pow([i as u64]))
            .collect::<Vec<_>>(),
    );

    end_timer!(h_time);

    // Compute the L-query
    let l_time = start_timer!(|| "Calculate L");
    let l_query = FixedBaseMSM::multi_scalar_mul::<E::G1Projective>(scalar_bits, g1_window, &g1_table, &l);
    let mut l_query = l_query[assembly.num_inputs..].to_vec();
    end_timer!(l_time);

    end_timer!(proving_key_time);

    // Generate R1CS verification key
    let verifying_key_time = start_timer!(|| "Generate the R1CS verification key");
    let gamma_g2 = g2_generator.mul(&gamma);
    let gamma_abc_g1 = FixedBaseMSM::multi_scalar_mul::<E::G1Projective>(scalar_bits, g1_window, &g1_table, &gamma_abc);

    drop(g1_table);

    end_timer!(verifying_key_time);

    let vk = VerifyingKey::<E> {
        alpha_g1: alpha_g1.into_affine(),
        beta_g2: beta_g2.into_affine(),
        gamma_g2: gamma_g2.into_affine(),
        delta_g2: delta_g2.into_affine(),
        gamma_abc_g1: cfg_iter!(gamma_abc_g1).map(|p| p.into_affine()).collect::<Vec<_>>(),
    };

    let batch_normalization_time = start_timer!(|| "Convert proving key elements to affine");
    E::G1Projective::batch_normalization(a_query.as_mut_slice());
    E::G1Projective::batch_normalization(b_g1_query.as_mut_slice());
    E::G2Projective::batch_normalization(b_g2_query.as_mut_slice());
    E::G1Projective::batch_normalization(h_query.as_mut_slice());
    E::G1Projective::batch_normalization(l_query.as_mut_slice());
    end_timer!(batch_normalization_time);

    Ok(Parameters {
        vk,
        beta_g1: beta_g1.into_affine(),
        delta_g1: delta_g1.into_affine(),
        a_query: a_query.into_iter().map(Into::into).collect(),
        b_g1_query: b_g1_query.into_iter().map(Into::into).collect(),
        b_g2_query: b_g2_query.into_iter().map(Into::into).collect(),
        h_query: h_query.into_iter().map(Into::into).collect(),
        l_query: l_query.into_iter().map(Into::into).collect(),
    })
}
