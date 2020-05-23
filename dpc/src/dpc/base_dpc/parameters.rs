use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_models::storage::Storage;
use snarkvm_models::algorithms::SNARK;
use snarkvm_utilities::bytes::FromBytes;

use std::{io::Result as IoResult, path::PathBuf};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct CircuitParameters<C: BaseDPCComponents> {
    pub account_commitment: C::AccountCommitment,
    pub record_commitment: C::RecordCommitment,
    pub predicate_verification_key_commitment: C::PredicateVerificationKeyCommitment,
    pub predicate_verification_key_hash: C::PredicateVerificationKeyHash,
    pub local_data_commitment: C::LocalDataCommitment,
    pub value_commitment: C::ValueCommitment,
    pub serial_number_nonce: C::SerialNumberNonceCRH,
    pub signature: C::Signature,
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

    pub fn serial_number_nonce_parameters(&self) -> &C::SerialNumberNonceCRH {
        &self.circuit_parameters.serial_number_nonce
    }

    pub fn signature_parameters(&self) -> &C::Signature {
        &self.circuit_parameters.signature
    }

    pub fn value_commitment_parameters(&self) -> &C::ValueCommitment {
        &self.circuit_parameters.value_commitment
    }

    pub fn store(&self, parameter_dir: &PathBuf) -> IoResult<()> {
        let circuit_dir = parameter_dir.join("circuit/");

        // Circuit Parameters

        let account_commitment_parameters_path = &circuit_dir.join("account_commitment.params");
        let record_commitment_parameters_path = &circuit_dir.join("record_commitment.params");
        let predicate_vk_commitment_path = &circuit_dir.join("predicate_vk_commitment.params");
        let predicate_vk_crh_path = &circuit_dir.join("predicate_vk_crh.params");
        let local_data_commitment_path = &circuit_dir.join("local_data_commitment.params");
        let value_commitment_path = &circuit_dir.join("value_commitment.params");
        let serial_number_commitment_path = &circuit_dir.join("serial_number_commitment.params");
        let signature_path = &circuit_dir.join("signature.params");

        let circuit_parameters = &self.circuit_parameters;

        circuit_parameters
            .account_commitment
            .store(account_commitment_parameters_path)?;
        circuit_parameters
            .record_commitment
            .store(record_commitment_parameters_path)?;
        circuit_parameters
            .predicate_verification_key_commitment
            .store(predicate_vk_commitment_path)?;
        circuit_parameters
            .predicate_verification_key_hash
            .store(predicate_vk_crh_path)?;
        circuit_parameters
            .local_data_commitment
            .store(local_data_commitment_path)?;
        circuit_parameters.value_commitment.store(value_commitment_path)?;
        circuit_parameters
            .serial_number_nonce
            .store(serial_number_commitment_path)?;
        circuit_parameters.signature.store(signature_path)?;

        // Predicate SNARK Parameters

        let predicate_snark_pk_path = &parameter_dir.join("predicate_snark.params");
        let predicate_snark_vk_path = &parameter_dir.join("predicate_snark_vk.params");
        let predicate_snark_proof_path = &parameter_dir.join("predicate_snark.proof");

        self.predicate_snark_parameters
            .proving_key
            .store(predicate_snark_pk_path)?;
        self.predicate_snark_parameters
            .verification_key
            .store(predicate_snark_vk_path)?;
        self.predicate_snark_parameters
            .proof
            .store(predicate_snark_proof_path)?;

        // Outer SNARK parameters

        let outer_snark_pk_path = &parameter_dir.join("outer_snark.params");
        let outer_snark_vk_path = &parameter_dir.join("outer_snark_vk.params");
        let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
            self.outer_snark_parameters.1.clone().into();

        if let Some(parameters) = &self.outer_snark_parameters.0 {
            parameters.store(outer_snark_pk_path)?;
        };
        outer_snark_vk.store(outer_snark_vk_path)?;

        // Inner SNARK parameters

        let inner_snark_pk_path = &parameter_dir.join("inner_snark.params");
        let inner_snark_vk_path = &parameter_dir.join("inner_snark_vk.params");
        let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
            self.inner_snark_parameters.1.clone().into();

        if let Some(parameters) = &self.inner_snark_parameters.0 {
            parameters.store(inner_snark_pk_path)?;
        };
        inner_snark_vk.store(inner_snark_vk_path)?;

        Ok(())
    }

    pub fn load(dir_path: &PathBuf, verify_only: bool) -> IoResult<Self> {
        let circuit_dir = dir_path.join("circuit/");

        // Circuit Parameters
        let circuit_parameters: CircuitParameters<C> = {
            let account_commitment_path = &circuit_dir.join("account_commitment.params");
            let record_commitment_path = &circuit_dir.join("record_commitment.params");
            let predicate_vk_commitment_path = &circuit_dir.join("predicate_vk_commitment.params");
            let predicate_vk_crh_path = &circuit_dir.join("predicate_vk_crh.params");
            let local_data_commitment_path = &circuit_dir.join("local_data_commitment.params");
            let value_commitment_path = &circuit_dir.join("value_commitment.params");
            let serial_number_commitment_path = &circuit_dir.join("serial_number_commitment.params");
            let signature_path = &circuit_dir.join("signature.params");

            let account_commitment = C::AccountCommitment::load(account_commitment_path)?;
            let record_commitment = C::RecordCommitment::load(record_commitment_path)?;
            let predicate_verification_key_commitment =
                C::PredicateVerificationKeyCommitment::load(predicate_vk_commitment_path)?;
            let predicate_verification_key_hash = C::PredicateVerificationKeyHash::load(predicate_vk_crh_path)?;
            let local_data_commitment = C::LocalDataCommitment::load(local_data_commitment_path)?;
            let value_commitment = C::ValueCommitment::load(value_commitment_path)?;
            let serial_number_nonce = C::SerialNumberNonceCRH::load(serial_number_commitment_path)?;
            let signature = C::Signature::load(signature_path)?;

            CircuitParameters::<C> {
                account_commitment,
                record_commitment,
                predicate_verification_key_commitment,
                predicate_verification_key_hash,
                local_data_commitment,
                value_commitment,
                serial_number_nonce,
                signature,
            }
        };

        // SNARK Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pk_path = &dir_path.join("predicate_snark.params");
            let predicate_snark_proof_path = &dir_path.join("predicate_snark.proof");

            let proving_key = <C::PredicateSNARK as SNARK>::ProvingParameters::load(predicate_snark_pk_path)?;
            let verification_key = proving_key.clone().into();
            let proof = <C::PredicateSNARK as SNARK>::Proof::load(predicate_snark_proof_path)?;

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
                false => Some(<C::OuterSNARK as SNARK>::ProvingParameters::load(outer_snark_pk_path)?),
            };

            let outer_snark_vk = <C::OuterSNARK as SNARK>::VerificationParameters::load(outer_snark_vk_path)?;
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
                false => Some(<C::InnerSNARK as SNARK>::ProvingParameters::load(inner_snark_pk_path)?),
            };

            let inner_snark_vk = <C::InnerSNARK as SNARK>::VerificationParameters::load(inner_snark_vk_path)?;
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
        let circuit_parameters: CircuitParameters<C> = {
            let account_commitment_parameters = include_bytes!["../../parameters/circuit/account_commitment.params"];
            let record_commitment_parameters = include_bytes!["../../parameters/circuit/record_commitment.params"];
            let predicate_vk_commitment_parameters = vec![];
            let predicate_vk_crh_parameters = include_bytes!["../../parameters/circuit/predicate_vk_crh.params"];
            let local_data_commitment_parameters =
                include_bytes!["../../parameters/circuit/local_data_commitment.params"];
            let value_commitment_parameters = include_bytes!["../../parameters/circuit/value_commitment.params"];
            let serial_number_commitment_parameters =
                include_bytes!["../../parameters/circuit/serial_number_commitment.params"];
            let signature_parameters = &include_bytes!["../../parameters/circuit/signature.params"];

            let account_commitment: C::AccountCommitment = FromBytes::read(&account_commitment_parameters[..])?;
            let record_commitment: C::RecordCommitment = FromBytes::read(&record_commitment_parameters[..])?;
            let predicate_verification_key_commitment: C::PredicateVerificationKeyCommitment =
                FromBytes::read(&predicate_vk_commitment_parameters[..])?;
            let predicate_verification_key_hash: C::PredicateVerificationKeyHash =
                FromBytes::read(&predicate_vk_crh_parameters[..])?;
            let local_data_commitment: C::LocalDataCommitment = FromBytes::read(&local_data_commitment_parameters[..])?;
            let value_commitment: C::ValueCommitment = FromBytes::read(&value_commitment_parameters[..])?;
            let serial_number_nonce: C::SerialNumberNonceCRH =
                FromBytes::read(&serial_number_commitment_parameters[..])?;
            let signature: C::Signature = FromBytes::read(&signature_parameters[..])?;

            CircuitParameters::<C> {
                account_commitment,
                record_commitment,
                predicate_verification_key_commitment,
                predicate_verification_key_hash,
                local_data_commitment,
                value_commitment,
                serial_number_nonce,
                signature,
            }
        };

        // SNARK Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pk_bytes = include_bytes!["../../parameters/predicate_snark.params"];
            let predicate_snark_proof_bytes = include_bytes!["../../parameters/predicate_snark.proof"];

            let proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters =
                FromBytes::read(&predicate_snark_pk_bytes[..])?;
            let verification_key = proving_key.clone().into();
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
