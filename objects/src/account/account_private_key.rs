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

use crate::account_format;
use snarkos_errors::objects::AccountError;
use snarkvm_algorithms::prf::Blake2s;
use snarkvm_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme, PRF},
    dpc::DPCComponents,
};
use snarkvm_utilities::{bytes_to_bits, to_bytes, FromBytes, ToBytes};

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
    pub seed: [u8; 32],
    // Derived private attributes from the seed.
    pub sk_sig: <C::AccountSignature as SignatureScheme>::PrivateKey,
    pub sk_prf: <C::PRF as PRF>::Seed,
    pub r_pk: <C::AccountCommitment as CommitmentScheme>::Randomness,
    pub r_pk_counter: u16,
    // This dummy flag is set to true for use in the `inner_snark` setup.
    #[derivative(Default(value = "true"))]
    pub is_dummy: bool,
}

impl<C: DPCComponents> AccountPrivateKey<C> {
    const INITIAL_R_PK_COUNTER: u16 = 2;
    const INPUT_SK_PRF: [u8; 32] = [
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    const INPUT_SK_SIG: [u8; 32] = [0u8; 32];

    /// Creates a new account private key.
    pub fn new<R: Rng>(
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
        rng: &mut R,
    ) -> Result<Self, AccountError> {
        // Sample randomly until a valid private key is found.
        loop {
            // Samples a random account private key seed.
            let seed: [u8; 32] = rng.gen();

            // Returns the private key if it is valid.
            match Self::from_seed(signature_parameters, commitment_parameters, &seed) {
                Ok(private_key) => return Ok(private_key),
                _ => continue,
            };
        }
    }

    /// Derives the account private key from a given seed and verifies it is well-formed.
    pub fn from_seed(
        signature_parameters: &C::AccountSignature,
        commitment_parameters: &C::AccountCommitment,
        seed: &[u8; 32],
    ) -> Result<Self, AccountError> {
        // Generate the SIG key pair.
        let sk_sig_bytes = Blake2s::evaluate(&seed, &Self::INPUT_SK_SIG)?;
        let sk_sig = <C::AccountSignature as SignatureScheme>::PrivateKey::read(&sk_sig_bytes[..])?;

        // Generate the PRF secret key.
        let sk_prf_bytes = Blake2s::evaluate(&seed, &Self::INPUT_SK_PRF)?;
        let sk_prf = <C::PRF as PRF>::Seed::read(&sk_prf_bytes[..])?;

        let mut r_pk_counter = Self::INITIAL_R_PK_COUNTER;
        loop {
            if r_pk_counter == u16::MAX {
                return Err(AccountError::InvalidPrivateKeySeed);
            }

            // Derive the private key by iterating on the r_pk counter
            let private_key = match Self::derive_r_pk(seed, r_pk_counter) {
                Ok(r_pk) => Self {
                    seed: *seed,
                    sk_sig: sk_sig.clone(),
                    sk_prf: sk_prf.clone(),
                    r_pk,
                    r_pk_counter,
                    is_dummy: false,
                },
                _ => {
                    r_pk_counter += 1;
                    continue;
                }
            };

            // Returns the private key if it is valid.
            match private_key.is_valid(signature_parameters, commitment_parameters) {
                true => return Ok(private_key),
                false => {
                    r_pk_counter += 1;
                    continue;
                }
            }
        }
    }

    /// Derives the account private key from a given seed and counter without verifying if it is well-formed.
    pub fn from_seed_and_counter_unchecked(seed: &[u8; 32], r_pk_counter: u16) -> Result<Self, AccountError> {
        // Generate the SIG key pair.
        let sk_sig_bytes = Blake2s::evaluate(&seed, &Self::INPUT_SK_SIG)?;
        let sk_sig = <C::AccountSignature as SignatureScheme>::PrivateKey::read(&sk_sig_bytes[..])?;

        // Generate the PRF secret key.
        let sk_prf_bytes = Blake2s::evaluate(&seed, &Self::INPUT_SK_PRF)?;
        let sk_prf = <C::PRF as PRF>::Seed::read(&sk_prf_bytes[..])?;

        // Generate the randomness rpk for the commitment scheme.
        let r_pk = Self::derive_r_pk(seed, r_pk_counter)?;

        Ok(Self {
            seed: *seed,
            sk_sig,
            sk_prf,
            r_pk,
            r_pk_counter,
            is_dummy: false,
        })
    }

    /// Generate the randomness rpk for the commitment scheme from a given seed and counter
    fn derive_r_pk(
        seed: &[u8; 32],
        counter: u16,
    ) -> Result<<C::AccountCommitment as CommitmentScheme>::Randomness, AccountError> {
        let mut r_pk_input = [0u8; 32];
        r_pk_input[0..2].copy_from_slice(&counter.to_le_bytes());

        // Generate the randomness rpk for the commitment scheme.
        let r_pk_bytes = Blake2s::evaluate(seed, &r_pk_input)?;
        let r_pk = <C::AccountCommitment as CommitmentScheme>::Randomness::read(&r_pk_bytes[..])?;

        Ok(r_pk)
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
    ) -> Result<<C::AccountEncryption as EncryptionScheme>::PrivateKey, AccountError> {
        let commitment = self.commit(signature_parameters, commitment_parameters)?;
        let decryption_key_bytes = to_bytes![commitment]?;

        // This operation implicitly enforces that the unused MSB bits
        // for the scalar field representation are correctly set to 0.
        let decryption_key = match self.is_dummy {
            true => <C::AccountEncryption as EncryptionScheme>::PrivateKey::default(),
            false => <C::AccountEncryption as EncryptionScheme>::PrivateKey::read(&decryption_key_bytes[..])?,
        };

        // This operation explicitly enforces that the unused MSB bits
        // for the scalar field representation are correctly set to 0.
        //
        // To simplify verification of this isomorphism from the base field
        // to the scalar field in the `inner_snark`, we additionally enforce
        // that the MSB bit of the scalar field is also set to 0.
        if !self.is_dummy {
            let account_decryption_key_bits = bytes_to_bits(&decryption_key_bytes[..]).collect::<Vec<_>>();
            let account_decryption_key_length = account_decryption_key_bits.len();

            let decryption_private_key_length = C::AccountEncryption::private_key_size_in_bits();
            assert!(decryption_private_key_length > 0);
            assert!(decryption_private_key_length <= account_decryption_key_length);

            for i in (decryption_private_key_length - 1)..account_decryption_key_length {
                let bit_index = account_decryption_key_length - i - 1;
                if account_decryption_key_bits[bit_index] {
                    return Err(AccountError::InvalidAccountCommitment);
                }
            }
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
        // Construct the commitment input for the account address.
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
        if data.len() != 43 {
            return Err(AccountError::InvalidByteLength(data.len()));
        }

        if data[0..9] != account_format::PRIVATE_KEY_PREFIX {
            return Err(AccountError::InvalidPrefixBytes(data[0..9].to_vec()));
        }

        let mut reader = &data[9..];
        let counter_bytes: [u8; 2] = FromBytes::read(&mut reader)?;
        let seed: [u8; 32] = FromBytes::read(&mut reader)?;

        Self::from_seed_and_counter_unchecked(&seed, u16::from_le_bytes(counter_bytes))
    }
}

impl<C: DPCComponents> fmt::Display for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut private_key = [0u8; 43];
        let prefix = account_format::PRIVATE_KEY_PREFIX;

        private_key[0..9].copy_from_slice(&prefix);
        private_key[9..11].copy_from_slice(&self.r_pk_counter.to_le_bytes());

        self.seed
            .write(&mut private_key[11..43])
            .expect("seed formatting failed");

        write!(f, "{}", private_key.to_base58())
    }
}

impl<C: DPCComponents> fmt::Debug for AccountPrivateKey<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "AccountPrivateKey {{ seed: {:?}, r_pk_counter: {:?} }}",
            self.seed, self.r_pk_counter
        )
    }
}
