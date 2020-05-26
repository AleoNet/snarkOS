use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_models::{algorithms::SNARK, parameters::Parameter};
use snarkos_parameters::*;
use snarkos_utilities::bytes::FromBytes;

use std::{fs, io::Result as IoResult, path::PathBuf};

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

        Ok(Self {
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
}

impl<C: BaseDPCComponents> PredicateSNARKParameters<C> {
    // TODO (howardwu): Why are we not preparing the VK here?
    pub fn load() -> IoResult<Self> {
        let proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters =
            From::from(FromBytes::read(PredicateSNARKPKParameters::load_bytes().as_slice())?);
        let verification_key = From::from(<C::PredicateSNARK as SNARK>::VerificationParameters::read(
            PredicateSNARKVKParameters::load_bytes().as_slice(),
        )?);

        Ok(Self {
            proving_key,
            verification_key,
        })
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct PublicParameters<C: BaseDPCComponents> {
    pub circuit_parameters: CircuitParameters<C>,
    pub predicate_snark_parameters: PredicateSNARKParameters<C>,
    pub inner_snark_parameters: (
        Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ),
    pub outer_snark_parameters: (
        Option<<C::OuterSNARK as SNARK>::ProvingParameters>,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
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
        // TODO (howardwu): Update logic to use parameters module.
        fn load_snark_pk<S: SNARK>(path: &PathBuf) -> IoResult<S::ProvingParameters> {
            let proving_parameters: <S as SNARK>::ProvingParameters = FromBytes::read(&fs::read(path)?[..])?;
            Ok(proving_parameters)
        }

        let circuit_parameters = CircuitParameters::<C>::load()?;
        let predicate_snark_parameters = PredicateSNARKParameters::<C>::load()?;

        let inner_snark_parameters = {
            // TODO (howardwu): Update logic to use parameters module.
            let inner_snark_pk_path = &dir_path.join("inner_snark.params");
            let inner_snark_pk = match verify_only {
                true => None,
                false => Some(load_snark_pk::<C::InnerSNARK>(inner_snark_pk_path)?),
            };

            let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
                From::from(<C::InnerSNARK as SNARK>::VerificationParameters::read(
                    InnerSNARKVKParameters::load_bytes().as_slice(),
                )?);

            (inner_snark_pk, inner_snark_vk.into())
        };

        let outer_snark_parameters = {
            // TODO (howardwu): Update logic to use parameters module.
            let outer_snark_pk_path = &dir_path.join("outer_snark.params");
            let outer_snark_pk = match verify_only {
                true => None,
                false => Some(load_snark_pk::<C::OuterSNARK>(outer_snark_pk_path)?),
            };

            let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
                From::from(<C::OuterSNARK as SNARK>::VerificationParameters::read(
                    OuterSNARKVKParameters::load_bytes().as_slice(),
                )?);

            (outer_snark_pk, outer_snark_vk.into())
        };

        Ok(Self {
            circuit_parameters,
            predicate_snark_parameters,
            inner_snark_parameters,
            outer_snark_parameters,
        })
    }

    pub fn load_vk_direct() -> IoResult<Self> {
        let circuit_parameters = CircuitParameters::<C>::load()?;
        let predicate_snark_parameters = PredicateSNARKParameters::<C>::load()?;

        let inner_snark_parameters = {
            let inner_snark_pk = None;
            let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
                From::from(<C::InnerSNARK as SNARK>::VerificationParameters::read(
                    InnerSNARKVKParameters::load_bytes().as_slice(),
                )?);
            (inner_snark_pk, inner_snark_vk.into())
        };

        let outer_snark_parameters = {
            let outer_snark_pk = None;
            let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
                From::from(<C::OuterSNARK as SNARK>::VerificationParameters::read(
                    OuterSNARKVKParameters::load_bytes().as_slice(),
                )?);
            (outer_snark_pk, outer_snark_vk.into())
        };

        Ok(Self {
            circuit_parameters,
            predicate_snark_parameters,
            inner_snark_parameters,
            outer_snark_parameters,
        })
    }
}
