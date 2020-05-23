use crate::snark::{
    create_random_proof,
    generate_random_parameters,
    prepare_verifying_key,
    verify_proof,
    Parameters,
    PreparedVerifyingKey,
    Proof,
    VerifyingKey,
};
use snarkos_models::{
    curves::{to_field_vec::ToConstraintField, PairingEngine},
    gadgets::r1cs::ConstraintSynthesizer,
};
use snarkvm_errors::algorithms::SNARKError;
use snarkvm_models::algorithms::SNARK;

use rand::Rng;
use std::marker::PhantomData;

/// Note: V should serialize its contents to `Vec<E::Fr>` in the same order as
/// during the constraint generation.
pub struct GM17<E: PairingEngine, C: ConstraintSynthesizer<E::Fr>, V: ToConstraintField<E::Fr> + ?Sized> {
    _engine: PhantomData<E>,
    _circuit: PhantomData<C>,
    _verifier_input: PhantomData<V>,
}

impl<E: PairingEngine, C: ConstraintSynthesizer<E::Fr>, V: ToConstraintField<E::Fr> + ?Sized> SNARK for GM17<E, C, V> {
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
        let setup_time = start_timer!(|| "{Groth-Maller 2017}::Setup");
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
        let proof_time = start_timer!(|| "{Groth-Maller 2017}::Prove");
        let result = create_random_proof::<E, _, _>(input_and_witness, pp, rng)?;
        end_timer!(proof_time);
        Ok(result)
    }

    fn verify(
        vk: &Self::PreparedVerificationParameters,
        input: &Self::VerifierInput,
        proof: &Self::Proof,
    ) -> Result<bool, SNARKError> {
        let verify_time = start_timer!(|| "{Groth-Maller 2017}::Verify");
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
