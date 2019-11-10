use crate::signature::SchnorrParameters;
use snarkos_errors::algorithms::Error;
use snarkos_models::{
    algorithms::SignatureScheme,
    curves::{Field, Group, PrimeField},
};
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};

use digest::Digest;
use rand::Rng;
use std::{hash::Hash, marker::PhantomData};

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for i in 0..8 {
            let bit = (*byte >> (8 - i - 1)) & 1;
            bits.push(bit == 1);
        }
    }
    bits
}

#[derive(Derivative)]
#[derivative(Clone(bound = "G: Group"), Default(bound = "G: Group"))]
pub struct SchnorrOutput<G: Group> {
    pub prover_response: G::ScalarField,
    pub verifier_challenge: G::ScalarField,
}

pub struct SchnorrSignature<G: Group, D: Digest> {
    _group: PhantomData<G>,
    _hash: PhantomData<D>,
}

impl<G: Group + Hash, D: Digest + Send + Sync> SignatureScheme for SchnorrSignature<G, D>
where
    G::ScalarField: PrimeField,
{
    type Output = SchnorrOutput<G>;
    type Parameters = SchnorrParameters<G, D>;
    type PrivateKey = G::ScalarField;
    type PublicKey = G;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self::Parameters, Error> {
        let setup_time = start_timer!(|| "SchnorrSig::Setup");

        let mut salt = [0u8; 32];
        rng.fill_bytes(&mut salt);
        let generator = G::rand(rng);

        end_timer!(setup_time);
        Ok(SchnorrParameters {
            _hash: PhantomData,
            generator,
            salt,
        })
    }

    fn keygen<R: Rng>(
        parameters: &Self::Parameters,
        rng: &mut R,
    ) -> Result<(Self::PublicKey, Self::PrivateKey), Error> {
        let keygen_time = start_timer!(|| "SchnorrSig::KeyGen");

        let private_key = G::ScalarField::rand(rng);
        let public_key = parameters.generator.mul(&private_key);

        end_timer!(keygen_time);
        Ok((public_key, private_key))
    }

    fn sign<R: Rng>(
        parameters: &Self::Parameters,
        private_key: &Self::PrivateKey,
        message: &[u8],
        rng: &mut R,
    ) -> Result<Self::Output, Error> {
        let sign_time = start_timer!(|| "SchnorrSig::Sign");
        // (k, e);
        let (random_scalar, verifier_challenge) = loop {
            // Sample a random scalar `k` from the prime scalar field.
            let random_scalar: G::ScalarField = G::ScalarField::rand(rng);
            // Commit to the random scalar via r := k · g.
            // This is the prover's first msg in the Sigma protocol.
            let prover_commitment: G = parameters.generator.mul(&random_scalar);

            // Hash everything to get verifier challenge.
            let mut hash_input = Vec::new();
            hash_input.extend_from_slice(&parameters.salt);
            hash_input.extend_from_slice(&to_bytes![prover_commitment]?);
            hash_input.extend_from_slice(message);

            // Compute the supposed verifier response: e := H(salt || r || msg);
            if let Some(verifier_challenge) = G::ScalarField::from_random_bytes(&D::digest(&hash_input)) {
                break (random_scalar, verifier_challenge);
            };
        };

        // k - xe;
        let prover_response = random_scalar - &(verifier_challenge * &private_key);
        let signature = SchnorrOutput {
            prover_response,
            verifier_challenge,
        };

        end_timer!(sign_time);
        Ok(signature)
    }

    fn verify(
        parameters: &Self::Parameters,
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Output,
    ) -> Result<bool, Error> {
        let verify_time = start_timer!(|| "SchnorrSig::Verify");

        let SchnorrOutput {
            prover_response,
            verifier_challenge,
        } = signature;
        let mut claimed_prover_commitment = parameters.generator.mul(prover_response);
        let public_key_times_verifier_challenge = public_key.mul(verifier_challenge);
        claimed_prover_commitment += &public_key_times_verifier_challenge;

        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(&parameters.salt);
        hash_input.extend_from_slice(&to_bytes![claimed_prover_commitment]?);
        hash_input.extend_from_slice(&message);

        let obtained_verifier_challenge =
            if let Some(obtained_verifier_challenge) = G::ScalarField::from_random_bytes(&D::digest(&hash_input)) {
                obtained_verifier_challenge
            } else {
                return Ok(false);
            };
        end_timer!(verify_time);
        Ok(verifier_challenge == &obtained_verifier_challenge)
    }

    fn randomize_public_key(
        parameters: &Self::Parameters,
        public_key: &Self::PublicKey,
        randomness: &[u8],
    ) -> Result<Self::PublicKey, Error> {
        let rand_pk_time = start_timer!(|| "SchnorrSig::RandomizePubKey");

        let mut randomized_pk = *public_key;
        let mut base = parameters.generator;
        let mut encoded = G::zero();
        for bit in bytes_to_bits(randomness) {
            if bit {
                encoded += &base;
            }
            base.double_in_place();
        }
        randomized_pk += &encoded;

        end_timer!(rand_pk_time);

        Ok(randomized_pk)
    }

    fn randomize_signature(
        _parameter: &Self::Parameters,
        signature: &Self::Output,
        randomness: &[u8],
    ) -> Result<Self::Output, Error> {
        let rand_signature_time = start_timer!(|| "SchnorrSig::RandomizeSig");
        let SchnorrOutput {
            prover_response,
            verifier_challenge,
        } = signature;
        let mut base = G::ScalarField::one();
        let mut multiplier = G::ScalarField::zero();
        for bit in bytes_to_bits(randomness) {
            if bit {
                multiplier += &base;
            }
            base.double_in_place();
        }

        let new_sig = SchnorrOutput {
            prover_response: *prover_response - &(*verifier_challenge * &multiplier),
            verifier_challenge: *verifier_challenge,
        };
        end_timer!(rand_signature_time);
        Ok(new_sig)
    }
}
