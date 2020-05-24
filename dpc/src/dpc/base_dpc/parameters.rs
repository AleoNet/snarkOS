use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_models::algorithms::SNARK;
use snarkos_parameters::*;
use snarkos_utilities::bytes::FromBytes;

use std::{fs::File, io::Result as IoResult, path::PathBuf};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct CircuitParameters<C: BaseDPCComponents> {
    pub account_commitment: C::AccountCommitment,
    pub account_signature: C::AccountSignature,
    pub record_commitment: C::RecordCommitment,
    pub predicate_verification_key_commitment: C::PredicateVerificationKeyCommitment,
    pub predicate_verification_key_hash: C::PredicateVerificationKeyHash,
    pub local_data_commitment: C::LocalDataCommitment,
    pub value_commitment: C::ValueCommitment,
    pub serial_number_nonce: C::SerialNumberNonceCRH,
}

impl<C: BaseDPCComponents> CircuitParameters<C> {
    // TODO (howardwu): Inspect what is going on with predicate_verification_key_commitment.
    pub fn load() -> IoResult<Self> {
        let account_commitment: C::AccountCommitment =
            From::from(FromBytes::read(AccountCommitmentParameters::load_bytes().as_slice())?);
        let account_signature: C::AccountSignature =
            From::from(FromBytes::read(AccountSignatureParameters::load_bytes().as_slice())?);
        let record_commitment: C::RecordCommitment =
            From::from(FromBytes::read(RecordCommitmentParameters::load_bytes().as_slice())?);
        let predicate_verification_key_commitment: C::PredicateVerificationKeyCommitment =
            From::from(FromBytes::read(vec![].as_slice())?);
        let predicate_verification_key_hash: C::PredicateVerificationKeyHash =
            From::from(FromBytes::read(PredicateVKCRHParameters::load_bytes().as_slice())?);
        let local_data_commitment: C::LocalDataCommitment =
            From::from(FromBytes::read(LocalDataCommitmentParameters::load_bytes().as_slice())?);
        let value_commitment: C::ValueCommitment =
            From::from(FromBytes::read(ValueCommitmentParameters::load_bytes().as_slice())?);
        let serial_number_nonce: C::SerialNumberNonceCRH = From::from(FromBytes::read(
            SerialNumberNonceCRHParameters::load_bytes().as_slice(),
        )?);

        Ok(CircuitParameters::<C> {
            account_commitment,
            account_signature,
            record_commitment,
            predicate_verification_key_commitment,
            predicate_verification_key_hash,
            local_data_commitment,
            value_commitment,
            serial_number_nonce,
        })
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct PredicateSNARKParameters<C: BaseDPCComponents> {
    pub proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters,
    pub verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateSNARK as SNARK>::Proof,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct PublicParameters<C: BaseDPCComponents> {
    pub circuit_parameters: CircuitParameters<C>,
    pub predicate_snark_parameters: PredicateSNARKParameters<C>,
    pub outer_snark_parameters: (
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ),
    pub inner_snark_parameters: (
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ),
}

impl<C: BaseDPCComponents> PublicParameters<C> {
    pub fn account_commitment_parameters(&self) -> &C::AccountCommitment {
        &self.circuit_parameters.account_commitment
    }

    pub fn account_signature_parameters(&self) -> &C::AccountSignature {
        &self.circuit_parameters.account_signature
    }

    pub fn inner_snark_parameters(
        &self,
    ) -> &(
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.inner_snark_parameters
    }

    pub fn local_data_commitment_parameters(&self) -> &C::LocalDataCommitment {
        &self.circuit_parameters.local_data_commitment
    }

    pub fn outer_snark_parameters(
        &self,
    ) -> &(
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.outer_snark_parameters
    }

    pub fn predicate_snark_parameters(&self) -> &PredicateSNARKParameters<C> {
        &self.predicate_snark_parameters
    }

    pub fn predicate_verification_key_commitment_parameters(&self) -> &C::PredicateVerificationKeyCommitment {
        &self.circuit_parameters.predicate_verification_key_commitment
    }

    pub fn predicate_verification_key_hash_parameters(&self) -> &C::PredicateVerificationKeyHash {
        &self.circuit_parameters.predicate_verification_key_hash
    }

    pub fn record_commitment_parameters(&self) -> &C::RecordCommitment {
        &self.circuit_parameters.record_commitment
    }

    pub fn value_commitment_parameters(&self) -> &C::ValueCommitment {
        &self.circuit_parameters.value_commitment
    }

    pub fn serial_number_nonce_parameters(&self) -> &C::SerialNumberNonceCRH {
        &self.circuit_parameters.serial_number_nonce
    }

    pub fn load(dir_path: &PathBuf, verify_only: bool) -> IoResult<Self> {
        fn load_snark_pk<S: SNARK>(path: &PathBuf) -> IoResult<S::ProvingParameters> {
            let mut file = File::open(path)?;
            let proving_parameters: <S as SNARK>::ProvingParameters = FromBytes::read(&mut file)?;
            Ok(proving_parameters)
        }

        fn load_snark_vk<S: SNARK>(path: &PathBuf) -> IoResult<S::VerificationParameters> {
            let mut file = File::open(path)?;
            let verification_parameters: <S as SNARK>::VerificationParameters = FromBytes::read(&mut file)?;
            Ok(verification_parameters)
        }

        fn load_snark_proof<S: SNARK>(path: &PathBuf) -> IoResult<S::Proof> {
            let mut file = File::open(path)?;
            let proof: <S as SNARK>::Proof = FromBytes::read(&mut file)?;
            Ok(proof)
        }

        // Circuit Parameters

        let circuit_parameters = CircuitParameters::<C>::load()?;

        // SNARK Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pk_path = &dir_path.join("predicate_snark.params");
            let predicate_snark_vk_path = &dir_path.join("predicate_snark_vk.params");
            let predicate_snark_proof_path = &dir_path.join("predicate_snark.proof");

            let proving_key = load_snark_pk::<C::PredicateSNARK>(predicate_snark_pk_path)?;
            let verification_key = load_snark_vk::<C::PredicateSNARK>(predicate_snark_vk_path)?;
            let proof = load_snark_proof::<C::PredicateSNARK>(predicate_snark_proof_path)?;

            PredicateSNARKParameters::<C> {
                proving_key,
                verification_key,
                proof,
            }
        };

        let outer_snark_parameters: (
            Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
            <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let outer_snark_pk_path = &dir_path.join("outer_snark.params");
            let outer_snark_vk_path = &dir_path.join("outer_snark_vk.params");

            let outer_snark_pk = match verify_only {
                true => None,
                false => Some(load_snark_pk::<C::OuterSNARK>(outer_snark_pk_path)?),
            };

            let outer_snark_vk = load_snark_vk::<C::OuterSNARK>(outer_snark_vk_path)?;
            let outer_snark_prepared_vk = outer_snark_vk.into();

            (outer_snark_pk, outer_snark_prepared_vk)
        };

        let inner_snark_parameters: (
            Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
            <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let inner_snark_pk_path = &dir_path.join("inner_snark.params");
            let inner_snark_vk_path = &dir_path.join("inner_snark_vk.params");

            let inner_snark_pk = match verify_only {
                true => None,
                false => Some(load_snark_pk::<C::InnerSNARK>(inner_snark_pk_path)?),
            };

            let inner_snark_vk = load_snark_vk::<C::InnerSNARK>(inner_snark_vk_path)?;
            let inner_snark_prepared_vk = inner_snark_vk.into();

            (inner_snark_pk, inner_snark_prepared_vk)
        };

        Ok(Self {
            circuit_parameters,
            predicate_snark_parameters,
            outer_snark_parameters,
            inner_snark_parameters,
        })
    }

    pub fn load_vk_direct() -> IoResult<Self> {
        // Circuit Parameters

        let circuit_parameters = CircuitParameters::<C>::load()?;

        // SNARK Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pk_bytes = include_bytes!["../../parameters/predicate_snark.params"];
            let predicate_snark_vk_bytes = include_bytes!["../../parameters/predicate_snark_vk.params"];
            let predicate_snark_proof_bytes = include_bytes!["../../parameters/predicate_snark.proof"];

            let proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters =
                FromBytes::read(&predicate_snark_pk_bytes[..])?;
            let verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters =
                FromBytes::read(&predicate_snark_vk_bytes[..])?;
            let proof: <C::PredicateSNARK as SNARK>::Proof = FromBytes::read(&predicate_snark_proof_bytes[..])?;

            PredicateSNARKParameters::<C> {
                proving_key,
                verification_key,
                proof,
            }
        };

        let outer_snark_parameters: (
            Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
            <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let outer_snark_pk = None;

            let outer_snark_vk_bytes = include_bytes!["../../parameters/outer_snark_vk.params"];
            let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
                FromBytes::read(&outer_snark_vk_bytes[..])?;
            let outer_snark_prepared_vk = outer_snark_vk.into();

            (outer_snark_pk, outer_snark_prepared_vk)
        };

        let inner_snark_parameters: (
            Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
            <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let inner_snark_pk = None;

            let inner_snark_vk_bytes = include_bytes!["../../parameters/inner_snark_vk.params"];
            let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
                FromBytes::read(&inner_snark_vk_bytes[..])?;
            let inner_snark_prepared_vk = inner_snark_vk.into();

            (inner_snark_pk, inner_snark_prepared_vk)
        };

        Ok(Self {
            circuit_parameters,
            predicate_snark_parameters,
            outer_snark_parameters,
            inner_snark_parameters,
        })
    }
}
