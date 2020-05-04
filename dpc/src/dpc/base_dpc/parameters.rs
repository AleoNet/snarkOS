use crate::dpc::base_dpc::BaseDPCComponents;
use snarkos_models::{algorithms::SNARK, storage::Storage};

use std::{io::Result as IoResult, path::PathBuf};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct CircuitParameters<C: BaseDPCComponents> {
    pub address_commitment_parameters: C::AddressCommitment,
    pub record_commitment_parameters: C::RecordCommitment,
    pub predicate_verification_key_commitment_parameters: C::PredicateVerificationKeyCommitment,
    pub predicate_verification_key_hash_parameters: C::PredicateVerificationKeyHash,
    pub local_data_commitment_parameters: C::LocalDataCommitment,
    pub value_commitment_parameters: C::ValueCommitment,
    pub serial_number_nonce_parameters: C::SerialNumberNonce,
    pub signature_parameters: C::Signature,
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
    pub fn address_commitment_parameters(&self) -> &C::AddressCommitment {
        &self.circuit_parameters.address_commitment_parameters
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
        &self.circuit_parameters.local_data_commitment_parameters
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
        &self.circuit_parameters.predicate_verification_key_commitment_parameters
    }

    pub fn predicate_verification_key_hash_parameters(&self) -> &C::PredicateVerificationKeyHash {
        &self.circuit_parameters.predicate_verification_key_hash_parameters
    }

    pub fn record_commitment_parameters(&self) -> &C::RecordCommitment {
        &self.circuit_parameters.record_commitment_parameters
    }

    pub fn serial_number_nonce_parameters(&self) -> &C::SerialNumberNonce {
        &self.circuit_parameters.serial_number_nonce_parameters
    }

    pub fn signature_parameters(&self) -> &C::Signature {
        &self.circuit_parameters.signature_parameters
    }

    pub fn value_commitment_parameters(&self) -> &C::ValueCommitment {
        &self.circuit_parameters.value_commitment_parameters
    }

    pub fn store(&self, parameter_dir: &PathBuf) -> IoResult<()> {
        let circuit_dir = parameter_dir.join("circuit/");

        // Circuit Parameters

        let address_comm_pp_path = &circuit_dir.join("address_commitment.params");
        let record_comm_pp_path = &circuit_dir.join("record_commitment.params");
        let predicate_vk_comm_pp_path = &circuit_dir.join("predicate_vk_commitment.params");
        let predicate_vk_crh_pp_path = &circuit_dir.join("predicate_vk_crh.params");
        let local_data_comm_pp_path = &circuit_dir.join("local_data_commitment.params");
        let value_comm_pp_path = &circuit_dir.join("value_commitment.params");
        let serial_number_comm_pp_path = &circuit_dir.join("serial_number_commitment.params");
        let signature_pp_path = &circuit_dir.join("signature.params");

        let circuit_parameters = &self.circuit_parameters;

        circuit_parameters
            .address_commitment_parameters
            .store(address_comm_pp_path)?;
        circuit_parameters
            .record_commitment_parameters
            .store(record_comm_pp_path)?;
        circuit_parameters
            .predicate_verification_key_commitment_parameters
            .store(predicate_vk_comm_pp_path)?;
        circuit_parameters
            .predicate_verification_key_hash_parameters
            .store(predicate_vk_crh_pp_path)?;
        circuit_parameters
            .local_data_commitment_parameters
            .store(local_data_comm_pp_path)?;
        circuit_parameters
            .value_commitment_parameters
            .store(value_comm_pp_path)?;
        circuit_parameters
            .serial_number_nonce_parameters
            .store(serial_number_comm_pp_path)?;
        circuit_parameters.signature_parameters.store(signature_pp_path)?;

        // Predicate SNARK Parameters

        let predicate_snark_pp_path = &parameter_dir.join("predicate_snark.params");
        let predicate_snark_vk_pp_path = &parameter_dir.join("predicate_snark_vk.params");
        let predicate_snark_proof_path = &parameter_dir.join("predicate_snark.proof");

        self.predicate_snark_parameters
            .proving_key
            .store(predicate_snark_pp_path)?;
        self.predicate_snark_parameters
            .verification_key
            .store(predicate_snark_vk_pp_path)?;
        self.predicate_snark_parameters
            .proof
            .store(predicate_snark_proof_path)?;

        // Outer SNARK parameters

        let outer_snark_pp_path = &parameter_dir.join("outer_snark.params");
        let outer_snark_vk_path = &parameter_dir.join("outer_snark_vk.params");
        let outer_snark_vk: <C::OuterSNARK as SNARK>::VerificationParameters =
            self.outer_snark_parameters.1.clone().into();

        if let Some(parameters) = &self.outer_snark_parameters.0 {
            parameters.store(outer_snark_pp_path)?;
        };
        outer_snark_vk.store(outer_snark_vk_path)?;

        // Inner SNARK parameters

        let inner_snark_pp_path = &parameter_dir.join("inner_snark.params");
        let inner_snark_vk_path = &parameter_dir.join("inner_snark_vk.params");
        let inner_snark_vk: <C::InnerSNARK as SNARK>::VerificationParameters =
            self.inner_snark_parameters.1.clone().into();

        if let Some(parameters) = &self.inner_snark_parameters.0 {
            parameters.store(inner_snark_pp_path)?;
        };
        inner_snark_vk.store(inner_snark_vk_path)?;

        Ok(())
    }

    pub fn load(dir_path: &PathBuf, verify_only: bool) -> IoResult<Self> {
        let circuit_dir = dir_path.join("circuit/");

        // Circuit Parameters
        let circuit_parameters: CircuitParameters<C> = {
            let address_comm_pp_path = &circuit_dir.join("address_commitment.params");
            let record_comm_pp_path = &circuit_dir.join("record_commitment.params");
            let predicate_vk_comm_pp_path = &circuit_dir.join("predicate_vk_commitment.params");
            let predicate_vk_crh_pp_path = &circuit_dir.join("predicate_vk_crh.params");
            let local_data_comm_pp_path = &circuit_dir.join("local_data_commitment.params");
            let value_comm_pp_path = &circuit_dir.join("value_commitment.params");
            let serial_number_comm_pp_path = &circuit_dir.join("serial_number_commitment.params");
            let signature_pp_path = &circuit_dir.join("signature.params");

            let address_commitment_parameters = C::AddressCommitment::load(address_comm_pp_path)?;
            let record_commitment_parameters = C::RecordCommitment::load(record_comm_pp_path)?;
            let predicate_verification_key_commitment_parameters =
                C::PredicateVerificationKeyCommitment::load(predicate_vk_comm_pp_path)?;
            let predicate_verification_key_hash_parameters =
                C::PredicateVerificationKeyHash::load(predicate_vk_crh_pp_path)?;
            let local_data_commitment_parameters = C::LocalDataCommitment::load(local_data_comm_pp_path)?;
            let value_commitment_parameters = C::ValueCommitment::load(value_comm_pp_path)?;
            let serial_number_nonce_parameters = C::SerialNumberNonce::load(serial_number_comm_pp_path)?;
            let signature_parameters = C::Signature::load(signature_pp_path)?;

            CircuitParameters::<C> {
                address_commitment_parameters,
                record_commitment_parameters,
                predicate_verification_key_commitment_parameters,
                predicate_verification_key_hash_parameters,
                local_data_commitment_parameters,
                value_commitment_parameters,
                serial_number_nonce_parameters,
                signature_parameters,
            }
        };

        // Snark Parameters

        let predicate_snark_parameters: PredicateSNARKParameters<C> = {
            let predicate_snark_pp_path = &dir_path.join("predicate_snark.params");
            let predicate_snark_proof_path = &dir_path.join("predicate_snark.proof");

            let proving_key = <C::PredicateSNARK as SNARK>::ProvingParameters::load(predicate_snark_pp_path)?;
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
            let outer_snark_path = &dir_path.join("outer_snark.params");
            let outer_snark_vk_path = &dir_path.join("outer_snark_vk.params");

            let outer_snark_pk = match verify_only {
                true => None,
                false => Some(<C::OuterSNARK as SNARK>::ProvingParameters::load(outer_snark_path)?),
            };

            let outer_snark_vk = <C::OuterSNARK as SNARK>::VerificationParameters::load(outer_snark_vk_path)?;
            let outer_snark_prepared_vk = outer_snark_vk.into();

            (outer_snark_pk, outer_snark_prepared_vk)
        };

        let inner_snark_parameters: (
            Option<<C::InnerSNARK as SNARK>::ProvingParameters>,
            <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
        ) = {
            let inner_snark_pp_path = &dir_path.join("inner_snark.params");
            let inner_snark_vk_path = &dir_path.join("inner_snark_vk.params");

            let inner_snark_pk = match verify_only {
                true => None,
                false => Some(<C::InnerSNARK as SNARK>::ProvingParameters::load(inner_snark_pp_path)?),
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
}
