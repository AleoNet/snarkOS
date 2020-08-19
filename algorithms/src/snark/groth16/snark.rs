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

use super::{
    create_random_proof,
    generate_random_parameters,
    prepare_verifying_key,
    verify_proof,
    Parameters,
    PreparedVerifyingKey,
    Proof,
    VerifyingKey,
};
use snarkos_errors::algorithms::SNARKError;
use snarkos_models::{
    algorithms::SNARK,
    curves::{to_field_vec::ToConstraintField, PairingEngine},
    gadgets::r1cs::ConstraintSynthesizer,
};

use rand::Rng;
use std::marker::PhantomData;

/// Note: V should serialize its contents to `Vec<E::Fr>` in the same order as
/// during the constraint generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Groth16<E: PairingEngine, C: ConstraintSynthesizer<E::Fr>, V: ToConstraintField<E::Fr> + ?Sized> {
    _engine: PhantomData<E>,
    _circuit: PhantomData<C>,
    _verifier_input: PhantomData<V>,
}

impl<E: PairingEngine, C: ConstraintSynthesizer<E::Fr>, V: ToConstraintField<E::Fr> + ?Sized> SNARK
    for Groth16<E, C, V>
{
    type AssignedCircuit = C;
    type Circuit = C;
    type PreparedVerificationParameters = PreparedVerifyingKey<E>;
    type Proof = Proof<E>;
    type ProvingParameters = Parameters<E>;
    type VerificationParameters = VerifyingKey<E>;
    type VerifierInput = V;

    fn setup<R: Rng>(
        circuit: Self::Circuit,
        rng: &mut R,
    ) -> Result<(Self::ProvingParameters, Self::PreparedVerificationParameters), SNARKError> {
        let setup_time = start_timer!(|| "{Groth 2016}::Setup");
        let pp = generate_random_parameters::<E, Self::Circuit, R>(circuit, rng)?;
        let vk = prepare_verifying_key(&pp.vk);
        end_timer!(setup_time);
        Ok((pp, vk))
    }

    fn prove<R: Rng>(
        pp: &Self::ProvingParameters,
        input_and_witness: Self::AssignedCircuit,
        rng: &mut R,
    ) -> Result<Self::Proof, SNARKError> {
        let proof_time = start_timer!(|| "{Groth 2016}::Prove");
        let result = create_random_proof::<E, _, _>(input_and_witness, pp, rng)?;
        end_timer!(proof_time);
        Ok(result)
    }

    fn verify(
        vk: &Self::PreparedVerificationParameters,
        input: &Self::VerifierInput,
        proof: &Self::Proof,
    ) -> Result<bool, SNARKError> {
        let verify_time = start_timer!(|| "{Groth 2016}::Verify");
        let conversion_time = start_timer!(|| "Convert input to E::Fr");
        let input = input.to_field_elements()?;
        end_timer!(conversion_time);
        let verification = start_timer!(|| format!("Verify proof w/ input len: {}", input.len()));
        let result = verify_proof(&vk, proof, &input)?;
        end_timer!(verification);
        end_timer!(verify_time);
        Ok(result)
    }
}
