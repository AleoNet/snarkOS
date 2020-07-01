use crate::account_format;
use snarkos_errors::objects::AccountError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, PRF},
    curves::PrimeField,
    dpc::DPCComponents,
};
use snarkos_utilities::{rand::UniformRand, to_bytes, BigInteger, FromBytes, ToBytes};

use base58::{FromBase58, ToBase58};
use rand::Rng;
use std::{fmt, str::FromStr};

#[derive(Derivative)]
#[derivative(
    Clone(bound = "C: DPCComponents"),
    Default(bound = "C: DPCComponents"),
    PartialEq(bound = "C: DPCComponents"),
    Eq(bound = "C: DPCComponents")
)]
pub struct AccountPrivateKey<C: DPCComponents> {
    pub sk_sig: <C::AccountSignature as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::PRF as PRF>::Seed,
    pub r_pk: <C::AccountCommitment as CommitmentScheme>::Randomness,
}

impl<C: DPCComponents> AccountPrivateKey<C> {
    /// Creates a new account private key.
    pub fn new<R: Rng>(
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
        rng: &mut R,
    ) -> Result<Self, AccountError> {
        // Sample SIG key pair.
        let sk_sig = C::AccountSignature::generate_private_key(signature_parameters, rng)?;

        // Sample PRF secret key.
        let sk_bytes: [u8; 32] = rng.gen();
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&sk_bytes[..])?;

        // Sample randomness rpk for the commitment scheme.
        let r_pk = <C::AccountCommitment as CommitmentScheme>::Randomness::rand(rng);

        // Construct the account private key.
        let mut private_key = Self { sk_sig, sk_prf, r_pk };

        // Sample randomly until a valid private key is found.
        loop {
            // Sample randomness rpk for the commitment scheme.
            private_key.r_pk = <C::AccountCommitment as CommitmentScheme>::Randomness::rand(rng);

            // Returns the private key if it is valid.
            if private_key.is_valid(signature_parameters, commitment_parameters) {
                return Ok(private_key);
            }
        }
    }

    /// Returns `true` if the private key is well-formed. Otherwise, returns `false`.
    pub fn is_valid(
        &self,
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
    ) -> bool {
        self.to_decryption_key(signature_parameters, commitment_parameters)
            .is_ok()
    }

    /// Returns the decryption key for the account view key.
    pub fn to_decryption_key(
        &self,
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
    ) -> Result<C::AccountDecryptionKey, AccountError> {
        let commitment_bytes = to_bytes![self.commit(signature_parameters, commitment_parameters)?]?;

        // This operation implicitly enforces that the unused MSB bits
        // for the scalar field representation are correctly set to 0.
        let decryption_key = C::AccountDecryptionKey::read(&commitment_bytes[..])?;

        // To simplify verification of this isomorphism from the base field
        // to the scalar field in the `inner_snark`, we additionally enforce
        // that the MSB bit of the scalar field is also set to 0.
        if decryption_key
            .into_repr()
            .get_bit(C::AccountDecryptionKey::size_in_bits() - 1)
        {
            return Err(AccountError::InvalidAccountCommitment);
        }

        Ok(decryption_key)
    }

    /// Returns the signature public key for deriving the account view key.
    pub fn pk_sig(
        &self,
        signature_parameters: &C::AccountSignature,
    ) -> Result<<C::AccountSignature as SignatureScheme>::PublicKey, AccountError> {
        Ok(C::AccountSignature::generate_public_key(
            signature_parameters,
            &self.sk_sig,
        )?)
    }

    /// Returns the commitment output of the private key.
    fn commit(
        &self,
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
    ) -> Result<<C::AccountCommitment as CommitmentScheme>::Output, AccountError> {
        // Construct the commitment input for the account public key.
        let pk_sig = self.pk_sig(signature_parameters)?;
        let commit_input = to_bytes![pk_sig, self.sk_prf]?;

        Ok(C::AccountCommitment::commit(
            commitment_parameters,
            &commit_input,
            &self.r_pk,
        )?)
    }
}

impl<C: DPCComponents> FromStr for AccountPrivateKey<C> {
    type Err = AccountError;

    /// Reads in an account private key string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let data = s.from_base58()?;
        if data.len() != 132 {
            return Err(AccountError::InvalidByteLength(data.len()));
        }

        if &data[0..4] != account_format::PRIVATE_KEY_PREFIX {
            return Err(AccountError::InvalidPrefixBytes(data[0..4].to_vec()));
        }

        let mut reader = &data[4..];
        let sk_sig: <C::AccountSignature as SignatureScheme>::PrivateKey = FromBytes::read(&mut reader)?;
        let sk_prf: <C::PRF as PRF>::Seed = FromBytes::read(&mut reader)?;
        let r_pk: <C::AccountCommitment as CommitmentScheme>::Randomness = FromBytes::read(&mut reader)?;

        Ok(Self { sk_sig, sk_prf, r_pk })
    }
}

impl<C: DPCComponents> fmt::Display for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut private_key = [0u8; 101];
        let prefix = account_format::PRIVATE_KEY_PREFIX;

        private_key[0..5].copy_from_slice(&prefix);

        self.sk_sig
            .write(&mut private_key[5..37])
            .expect("sk_sig formatting failed");
        self.sk_prf
            .write(&mut private_key[37..69])
            .expect("sk_prf formatting failed");
        self.r_pk
            .write(&mut private_key[69..101])
            .expect("r_pk formatting failed");

        write!(f, "{}", private_key.to_base58())
    }
}

impl<C: DPCComponents> fmt::Debug for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AccountPrivateKey {{ sk_sig: {:?}, sk_prf: {:?}, r_pk: {:?} }}",
            self.sk_sig, self.sk_prf, self.r_pk,
        )
    }
}
