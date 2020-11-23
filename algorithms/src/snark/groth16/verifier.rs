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

use super::{PreparedVerifyingKey, Proof, VerifyingKey};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::curves::{AffineCurve, PairingCurve, PairingEngine, PrimeField, ProjectiveCurve};

use core::ops::{AddAssign, Neg};

pub fn prepare_verifying_key<E: PairingEngine>(vk: &VerifyingKey<E>) -> PreparedVerifyingKey<E> {
    PreparedVerifyingKey {
        vk: vk.clone(),
        alpha_g1_beta_g2: E::pairing(vk.alpha_g1, vk.beta_g2),
        gamma_g2_neg_pc: vk.gamma_g2.neg().prepare(),
        delta_g2_neg_pc: vk.delta_g2.neg().prepare(),
        gamma_abc_g1: vk.gamma_abc_g1.clone(),
    }
}

pub fn verify_proof<E: PairingEngine>(
    pvk: &PreparedVerifyingKey<E>,
    proof: &Proof<E>,
    public_inputs: &[E::Fr],
) -> Result<bool, SynthesisError> {
    if (public_inputs.len() + 1) != pvk.gamma_abc_g1.len() {
        return Err(SynthesisError::MalformedVerifyingKey);
    }

    let mut g_ic = pvk.gamma_abc_g1[0].into_projective();
    for (i, b) in public_inputs.iter().zip(pvk.gamma_abc_g1.iter().skip(1)) {
        g_ic.add_assign(&b.mul(i.into_repr()));
    }

    let qap = E::miller_loop(
        [
            (&proof.a.prepare(), &proof.b.prepare()),
            (&g_ic.into_affine().prepare(), &pvk.gamma_g2_neg_pc),
            (&proof.c.prepare(), &pvk.delta_g2_neg_pc),
        ]
        .iter()
        .copied(),
    );

    let test = E::final_exponentiation(&qap).ok_or(SynthesisError::UnexpectedIdentity)?;

    Ok(test == pvk.alpha_g1_beta_g2)
}
