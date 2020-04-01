use crate::snark::{PreparedVerifyingKey, Proof, VerifyingKey};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::curves::{AffineCurve, Field, PairingCurve, PairingEngine, PrimeField, ProjectiveCurve};

use std::ops::{AddAssign, MulAssign, Neg};

pub fn prepare_verifying_key<E: PairingEngine>(vk: &VerifyingKey<E>) -> PreparedVerifyingKey<E> {
    PreparedVerifyingKey {
        vk: vk.clone(),
        g_alpha: vk.g_alpha_g1,
        h_beta: vk.h_beta_g2,
        g_alpha_h_beta_ml: E::miller_loop([(&vk.g_alpha_g1.prepare(), &vk.h_beta_g2.prepare())].iter()),
        g_gamma_pc: vk.g_gamma_g1.prepare(),
        h_gamma_pc: vk.h_gamma_g2.prepare(),
        h_pc: vk.h_g2.prepare(),
        query: vk.query.clone(),
    }
}

pub fn verify_proof<E: PairingEngine>(
    pvk: &PreparedVerifyingKey<E>,
    proof: &Proof<E>,
    public_inputs: &[E::Fr],
) -> Result<bool, SynthesisError> {
    println!("public_inputs.len() + 1: {:?}", public_inputs.len() + 1);
    println!("pvk.query.len(): {:?}", pvk.query.len());

    if (public_inputs.len() + 1) != pvk.query.len() {
        return Err(SynthesisError::MalformedVerifyingKey);
    }

    // e(A*G^{alpha}, B*H^{beta}) = e(G^{alpha}, H^{beta}) * e(G^{psi}, H^{gamma}) *
    // e(C, H) where psi = \sum_{i=0}^l input_i pvk.query[i]

    let mut g_psi = pvk.query[0].into_projective();
    for (i, b) in public_inputs.iter().zip(pvk.query.iter().skip(1)) {
        g_psi.add_assign(&b.mul(i.into_repr()));
    }

    let mut test1_a_g_alpha = proof.a.into_projective();
    test1_a_g_alpha.add_assign(&pvk.g_alpha.into_projective());
    let test1_a_g_alpha = test1_a_g_alpha.into_affine();

    let mut test1_b_h_beta = proof.b.into_projective();
    test1_b_h_beta.add_assign(&pvk.h_beta.into_projective());
    let test1_b_h_beta = test1_b_h_beta.into_affine();

    let test1_r1 = pvk.g_alpha_h_beta_ml;
    let test1_r2 = E::miller_loop(
        [
            (&test1_a_g_alpha.neg().prepare(), &test1_b_h_beta.prepare()),
            (&g_psi.into_affine().prepare(), &pvk.h_gamma_pc),
            (&proof.c.prepare(), &pvk.h_pc),
        ]
        .iter(),
    );
    let mut test1_exp = test1_r2;
    test1_exp.mul_assign(&test1_r1);

    let test1 = E::final_exponentiation(&test1_exp).unwrap();

    // e(A, H^{gamma}) = e(G^{gamma}, B)

    let test2_exp = E::miller_loop(
        [
            (&proof.a.prepare(), &pvk.h_gamma_pc),
            (&pvk.g_gamma_pc, &proof.b.neg().prepare()),
        ]
        .iter(),
    );

    let test2 = E::final_exponentiation(&test2_exp).unwrap();

    Ok(test1 == E::Fqk::one() && test2 == E::Fqk::one())
}
