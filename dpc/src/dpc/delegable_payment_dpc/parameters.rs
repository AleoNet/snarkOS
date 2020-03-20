use crate::dpc::delegable_payment_dpc::DelegablePaymentDPCComponents;

use snarkos_models::algorithms::{SignatureScheme, SNARK};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct CommCRHSigPublicParameters<C: DelegablePaymentDPCComponents> {
    pub addr_comm_pp: C::AddrC,
    pub rec_comm_pp: C::RecC,
    pub pred_vk_comm_pp: C::PredVkComm,
    pub local_data_comm_pp: C::LocalDataComm,
    pub value_comm_pp: C::ValueComm,

    pub sn_nonce_crh_pp: C::SnNonceH,
    pub pred_vk_crh_pp: C::PredVkH,

    pub sig_pp: <C::S as SignatureScheme>::Parameters,
}

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct PredNIZKParameters<C: DelegablePaymentDPCComponents> {
    pub pk: <C::PredicateNIZK as SNARK>::ProvingParameters,
    pub vk: <C::PredicateNIZK as SNARK>::VerificationParameters,
    pub proof: <C::PredicateNIZK as SNARK>::Proof,
}

pub struct PublicParameters<C: DelegablePaymentDPCComponents> {
    pub comm_crh_sig_pp: CommCRHSigPublicParameters<C>,
    pub pred_nizk_pp: PredNIZKParameters<C>,
    pub proof_check_nizk_pp: (
        <C::ProofCheckNIZK as SNARK>::ProvingParameters,
        <C::ProofCheckNIZK as SNARK>::PreparedVerificationParameters,
    ),
    pub core_nizk_pp: (
        <C::MainNIZK as SNARK>::ProvingParameters,
        <C::MainNIZK as SNARK>::PreparedVerificationParameters,
    ),
}

impl<C: DelegablePaymentDPCComponents> PublicParameters<C> {
    pub fn core_check_nizk_pp(
        &self,
    ) -> &(
        <C::MainNIZK as SNARK>::ProvingParameters,
        <C::MainNIZK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.core_nizk_pp
    }

    pub fn proof_check_nizk_pp(
        &self,
    ) -> &(
        <C::ProofCheckNIZK as SNARK>::ProvingParameters,
        <C::ProofCheckNIZK as SNARK>::PreparedVerificationParameters,
    ) {
        &self.proof_check_nizk_pp
    }

    pub fn pred_nizk_pp(&self) -> &PredNIZKParameters<C> {
        &self.pred_nizk_pp
    }

    pub fn sn_nonce_crh_pp(&self) -> &C::SnNonceH {
        &self.comm_crh_sig_pp.sn_nonce_crh_pp
    }

    pub fn pred_vk_crh_pp(&self) -> &C::PredVkH {
        &self.comm_crh_sig_pp.pred_vk_crh_pp
    }

    pub fn local_data_comm_pp(&self) -> &C::LocalDataComm {
        &self.comm_crh_sig_pp.local_data_comm_pp
    }

    pub fn addr_comm_pp(&self) -> &C::AddrC {
        &self.comm_crh_sig_pp.addr_comm_pp
    }

    pub fn rec_comm_pp(&self) -> &C::RecC {
        &self.comm_crh_sig_pp.rec_comm_pp
    }

    pub fn pred_vk_comm_pp(&self) -> &C::PredVkComm {
        &self.comm_crh_sig_pp.pred_vk_comm_pp
    }

    pub fn value_comm_pp(&self) -> &C::ValueComm {
        &self.comm_crh_sig_pp.value_comm_pp
    }

    pub fn sig_pp(&self) -> &<C::S as SignatureScheme>::Parameters {
        &self.comm_crh_sig_pp.sig_pp
    }
}
