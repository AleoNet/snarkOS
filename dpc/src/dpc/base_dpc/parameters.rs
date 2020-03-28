use crate::dpc::base_dpc::BaseDPCComponents;

use snarkos_models::algorithms::{CommitmentScheme, SignatureScheme, CRH, SNARK};

use std::io::Result as IoResult;

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

pub struct PublicParameters<C: BaseDPCComponents> {
    pub circuit_parameters: CircuitParameters<C>,
    pub predicate_snark_parameters: PredicateSNARKParameters<C>,
    pub outer_snark_parameters: (
        <C::OuterSNARK as SNARK>::ProvingParameters,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ),
    pub inner_snark_parameters: (
        <C::InnerSNARK as SNARK>::ProvingParameters,
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
        <C::InnerSNARK as SNARK>::ProvingParameters,
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
        <C::OuterSNARK as SNARK>::ProvingParameters,
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

    pub fn store(&self) -> IoResult<()> {
        let mut parameter_dir = std::env::current_dir()?;
        parameter_dir.push("../parameters/");

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

        // Snark Parameters

        // TODO Store snark parameters - Add a Store/Load trait instead of adding store and load fn to each trait
        //
        //        let predicate_snark_pp_path = &circuit_dir.join("predicate_snark.params");
        //        let outer_snark_path = &circuit_dir.join("outer_snark.params");
        //        let inner_snark_pp_path = &circuit_dir.join("inner_snark.params");
        //
        //        self.predicate_snark_parameters.proving_key.store(predicate_snark_pp_path)?;
        //        self.outer_snark_parameters.0.store(outer_snark_path)?;
        //        self.inner_snark_parameters.0.store(inner_snark_pp_path)?;

        Ok(())
    }

    //    pub fn load(dir_path: &PathBuf) -> IoResult<Self> {
    //
    //        let circuit_dir = dir_path.join("circuit/");
    //
    //        // Circuit Parameters
    //
    //        let address_comm_pp_path = &circuit_dir.join("address_commitment.params");
    //        let record_comm_pp_path = &circuit_dir.join("record_commitment.params");
    //        let predicate_vk_comm_pp_path = &circuit_dir.join("predicate_vk_commitment.params");
    //        let predicate_vk_crh_pp_path = &circuit_dir.join("predicate_vk_crh.params");
    //        let local_data_comm_pp_path = &circuit_dir.join("local_data_commitment.params");
    //        let value_comm_pp_path = &circuit_dir.join("value_commitment.params");
    //        let serial_number_comm_pp_path = &circuit_dir.join("serial_number_commitment.params");
    //        let signature_pp_path = &circuit_dir.join("signature.params");
    //
    //    }
}
