use crate::signature::SchnorrParameters;
use snarkos_errors::{algorithms::SignatureError, curves::ConstraintFieldError};
use snarkos_models::{
    algorithms::SignatureScheme,
    curves::{to_field_vec::ToConstraintField, Field, Group, One, PrimeField, Zero},
};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    rand::UniformRand,
    to_bytes,
};

use digest::Digest;
use rand::Rng;
use std::{
    hash::Hash,
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
};

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for i in 0..8 {
            let bit = (*byte >> i) & 1;
            bits.push(bit == 1);
        }
    }
    bits
}

#[derive(Derivative)]
#[derivative(Clone(bound = "G: Group"), Debug(bound = "G: Group"), Default(bound = "G: Group"))]
pub struct SchnorrOutput<G: Group> {
    pub prover_response: <G as Group>::ScalarField,
    pub verifier_challenge: <G as Group>::ScalarField,
}

impl<G: Group> ToBytes for SchnorrOutput<G> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.prover_response.write(&mut writer)?;
        self.verifier_challenge.write(&mut writer)
    }
}

impl<G: Group> FromBytes for SchnorrOutput<G> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let prover_response = <G as Group>::ScalarField::read(&mut reader)?;
        let verifier_challenge = <G as Group>::ScalarField::read(&mut reader)?;

        Ok(Self {
            prover_response,
            verifier_challenge,
        })
    }
}

#[derive(Derivative)]
#[derivative(
    Copy(bound = "G: Group"),
    Clone(bound = "G: Group"),
    PartialEq(bound = "G: Group"),
    Eq(bound = "G: Group"),
    Debug(bound = "G: Group"),
    Hash(bound = "G: Group"),
    Default(bound = "G: Group")
)]
pub struct SchnorrPublicKey<G: Group>(pub G);

impl<G: Group> ToBytes for SchnorrPublicKey<G> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.0.write(&mut writer)
    }
}

impl<G: Group> FromBytes for SchnorrPublicKey<G> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        Ok(Self(G::read(&mut reader)?))
    }
}

impl<F: Field, G: Group + ToConstraintField<F>> ToConstraintField<F> for SchnorrPublicKey<G> {
    #[inline]
    fn to_field_elements(&self) -> Result<Vec<F>, ConstraintFieldError> {
        self.0.to_field_elements()
    }
}

#[derive(Derivative)]
#[derivative(
    Clone(bound = "G: Group, D: Digest"),
    Debug(bound = "G: Group, D: Digest"),
    PartialEq(bound = "G: Group, D: Digest"),
    Eq(bound = "G: Group, D: Digest")
)]
pub struct SchnorrSignature<G: Group, D: Digest> {
    pub parameters: SchnorrParameters<G, D>,
}

impl<G: Group + Hash, D: Digest + Send + Sync> SignatureScheme for SchnorrSignature<G, D>
where
    <G as Group>::ScalarField: PrimeField,
{
    type Output = SchnorrOutput<G>;
    type Parameters = SchnorrParameters<G, D>;
    type PrivateKey = <G as Group>::ScalarField;
    type PublicKey = SchnorrPublicKey<G>;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self, SignatureError> {
        let setup_time = start_timer!(|| "SchnorrSignature::setup");

        let mut salt = [0u8; 32];
        rng.fill_bytes(&mut salt);
        let generator = G::rand(rng);

        end_timer!(setup_time);

        let parameters = SchnorrParameters {
            _hash: PhantomData,
            generator,
            salt,
        };

        Ok(Self { parameters })
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.parameters
    }

    fn generate_private_key<R: Rng>(&self, rng: &mut R) -> Result<Self::PrivateKey, SignatureError> {
        let keygen_time = start_timer!(|| "SchnorrSignature::generate_private_key");
        let private_key = <G as Group>::ScalarField::rand(rng);
        end_timer!(keygen_time);

        Ok(private_key)
    }

    fn generate_public_key(&self, private_key: &Self::PrivateKey) -> Result<Self::PublicKey, SignatureError> {
        let keygen_time = start_timer!(|| "SchnorrSignature::generate_public_key");
        let public_key = self.parameters.generator.mul(private_key);
        end_timer!(keygen_time);

        Ok(SchnorrPublicKey(public_key))
    }

    fn sign<R: Rng>(
        &self,
        private_key: &Self::PrivateKey,
        message: &[u8],
        rng: &mut R,
    ) -> Result<Self::Output, SignatureError> {
        let sign_time = start_timer!(|| "SchnorrSignature::sign");
        // (k, e);
        let (random_scalar, verifier_challenge) = loop {
            // Sample a random scalar `k` from the prime scalar field.
            let random_scalar: <G as Group>::ScalarField = <G as Group>::ScalarField::rand(rng);
            // Commit to the random scalar via r := k · g.
            // This is the prover's first msg in the Sigma protocol.
            let prover_commitment: G = self.parameters.generator.mul(&random_scalar);

            // Hash everything to get verifier challenge.
            let mut hash_input = Vec::new();
            hash_input.extend_from_slice(&self.parameters.salt);
            hash_input.extend_from_slice(&to_bytes![prover_commitment]?);
            hash_input.extend_from_slice(message);

            // Compute the supposed verifier response: e := H(salt || r || msg);
            if let Some(verifier_challenge) = <G as Group>::ScalarField::from_random_bytes(&D::digest(&hash_input)) {
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
        &self,
        public_key: &Self::PublicKey,
        message: &[u8],
        signature: &Self::Output,
    ) -> Result<bool, SignatureError> {
        let verify_time = start_timer!(|| "SchnorrSignature::Verify");

        let SchnorrOutput {
            prover_response,
            verifier_challenge,
        } = signature;
        let mut claimed_prover_commitment = self.parameters.generator.mul(prover_response);
        let public_key_times_verifier_challenge = public_key.0.mul(verifier_challenge);
        claimed_prover_commitment += &public_key_times_verifier_challenge;

        let mut hash_input = Vec::new();
        hash_input.extend_from_slice(&self.parameters.salt);
        hash_input.extend_from_slice(&to_bytes![claimed_prover_commitment]?);
        hash_input.extend_from_slice(&message);

        let obtained_verifier_challenge = if let Some(obtained_verifier_challenge) =
            <G as Group>::ScalarField::from_random_bytes(&D::digest(&hash_input))
        {
            obtained_verifier_challenge
        } else {
            return Ok(false);
        };
        end_timer!(verify_time);
        Ok(verifier_challenge == &obtained_verifier_challenge)
    }

    fn randomize_public_key(
        &self,
        public_key: &Self::PublicKey,
        randomness: &[u8],
    ) -> Result<Self::PublicKey, SignatureError> {
        let rand_pk_time = start_timer!(|| "SchnorrSignature::randomize_public_key");

        let mut randomized_pk = public_key.0.clone();
        let mut base = self.parameters.generator;
        let mut encoded = G::zero();
        for bit in bytes_to_bits(randomness) {
            if bit {
                encoded += &base;
            }
            <G as Group>::double_in_place(&mut base);
        }
        randomized_pk += &encoded;

        end_timer!(rand_pk_time);

        Ok(SchnorrPublicKey(randomized_pk))
    }

    fn randomize_signature(&self, signature: &Self::Output, randomness: &[u8]) -> Result<Self::Output, SignatureError> {
        let rand_signature_time = start_timer!(|| "SchnorrSignature::randomize_signature");
        let SchnorrOutput {
            prover_response,
            verifier_challenge,
        } = signature;
        let mut base = <G as Group>::ScalarField::one();
        let mut multiplier = <G as Group>::ScalarField::zero();
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

impl<G: Group, D: Digest> From<SchnorrParameters<G, D>> for SchnorrSignature<G, D> {
    fn from(parameters: SchnorrParameters<G, D>) -> Self {
        Self { parameters }
    }
}
