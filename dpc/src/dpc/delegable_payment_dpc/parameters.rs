use crate::dpc::delegable_payment_dpc::DelegablePaymentDPCComponents;

use snarkos_models::algorithms::{SignatureScheme, SNARK};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct CircuitParameters<C: DelegablePaymentDPCComponents> {
    pub address_commitment_parameters: C::AddressCommitment,
    pub record_commitment_parameters: C::RecordCommitment,
    pub predicate_verification_key_commitment_parameters: C::PredicateVerificationKeyCommitment,
    pub predicate_verification_key_hash_parameters: C::PredicateVerificationKeyHash,
    pub local_data_commitment_parameters: C::LocalDataCommitment,
    pub value_commitment_parameters: C::ValueCommitment,
    pub serial_number_nonce_parameters: C::SerialNumberNonce,
    pub signature_parameters: <C::Signature as SignatureScheme>::Parameters,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct PredicateSNARKParameters<C: DelegablePaymentDPCComponents> {
    pub proving_key: <C::PredicateSNARK as SNARK>::ProvingParameters,
    pub verification_key: <C::PredicateSNARK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateSNARK as SNARK>::Proof,
}

pub struct PublicParameters<C: DelegablePaymentDPCComponents> {
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

impl<C: DelegablePaymentDPCComponents> PublicParameters<C> {
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

    pub fn signature_parameters(&self) -> &<C::Signature as SignatureScheme>::Parameters {
        &self.circuit_parameters.signature_parameters
    }

    pub fn value_commitment_parameters(&self) -> &C::ValueCommitment {
        &self.circuit_parameters.value_commitment_parameters
    }
}
