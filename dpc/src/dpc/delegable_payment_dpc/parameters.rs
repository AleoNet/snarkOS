use crate::dpc::delegable_payment_dpc::DelegablePaymentDPCComponents;

use snarkos_models::algorithms::{SignatureScheme, SNARK};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct CommCRHSigPublicParameters<C: DelegablePaymentDPCComponents> {
    pub addr_comm_pp: C::AddressCommitment,
    pub rec_comm_pp: C::RecordCommitment,
    pub pred_vk_comm_pp: C::PredicateVerificationKeyCommitment,
    pub local_data_comm_pp: C::LocalDataCommitment,
    pub value_comm_pp: C::ValueComm,

    pub sn_nonce_crh_pp: C::SerialNumberNonce,
    pub pred_vk_crh_pp: C::PredicateVerificationKeyHash,

    pub sig_pp: <C::Signature as SignatureScheme>::Parameters,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct PredicateSNARKParameters<C: DelegablePaymentDPCComponents> {
    pub pk: <C::PredicateSNARK as SNARK>::ProvingParameters,
    pub vk: <C::PredicateSNARK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateSNARK as SNARK>::Proof,
}

pub struct PublicParameters<C: DelegablePaymentDPCComponents> {
    pub comm_crh_sig_pp: CommCRHSigPublicParameters<C>,
    pub pred_nizk_pp: PredicateSNARKParameters<C>,
    pub proof_check_nizk_pp: (
        <C::OuterSNARK as SNARK>::ProvingParameters,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ),
    pub core_nizk_pp: (
        <C::InnerSNARK as SNARK>::ProvingParameters,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ),
}

impl<C: DelegablePaymentDPCComponents> PublicParameters<C> {
    pub fn core_check_nizk_pp(
        &self,
    ) -> &(
        <C::InnerSNARK as SNARK>::ProvingParameters,
        <C::InnerSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.core_nizk_pp
    }

    pub fn proof_check_nizk_pp(
        &self,
    ) -> &(
        <C::OuterSNARK as SNARK>::ProvingParameters,
        <C::OuterSNARK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.proof_check_nizk_pp
    }

    pub fn pred_nizk_pp(&self) -> &PredicateSNARKParameters<C> {
        &self.pred_nizk_pp
    }

    pub fn sn_nonce_crh_pp(&self) -> &C::SerialNumberNonce {
        &self.comm_crh_sig_pp.sn_nonce_crh_pp
    }

    pub fn pred_vk_crh_pp(&self) -> &C::PredicateVerificationKeyHash {
        &self.comm_crh_sig_pp.pred_vk_crh_pp
    }

    pub fn local_data_comm_pp(&self) -> &C::LocalDataCommitment {
        &self.comm_crh_sig_pp.local_data_comm_pp
    }

    pub fn addr_comm_pp(&self) -> &C::AddressCommitment {
        &self.comm_crh_sig_pp.addr_comm_pp
    }

    pub fn rec_comm_pp(&self) -> &C::RecordCommitment {
        &self.comm_crh_sig_pp.rec_comm_pp
    }

    pub fn pred_vk_comm_pp(&self) -> &C::PredicateVerificationKeyCommitment {
        &self.comm_crh_sig_pp.pred_vk_comm_pp
    }

    pub fn value_comm_pp(&self) -> &C::ValueComm {
        &self.comm_crh_sig_pp.value_comm_pp
    }

    pub fn sig_pp(&self) -> &<C::Signature as SignatureScheme>::Parameters {
        &self.comm_crh_sig_pp.sig_pp
    }
}
