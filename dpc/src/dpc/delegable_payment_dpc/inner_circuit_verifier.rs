use crate::{
    dpc::delegable_payment_dpc::{
        binding_signature::BindingSignature,
        parameters::CommCRHSigPublicParameters,
        DelegablePaymentDPCComponents,
    },
    ledger::MerkleTreeParams,
};
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerkleTreeDigest};
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};

pub struct InnerCircuitVerifier<C: DelegablePaymentDPCComponents> {
    // Commitment and CRH parameters
    pub comm_crh_sig_pp: CommCRHSigPublicParameters<C>,

    // Ledger parameters and digest
    pub ledger_pp: MerkleTreeParams<C::MerkleParameters>,
    pub ledger_digest: MerkleTreeDigest<C::MerkleParameters>,

    // Input record serial numbers and death predicate commitments
    pub old_serial_numbers: Vec<<C::Signature as SignatureScheme>::PublicKey>,

    // Output record commitments and birth predicate commitments
    pub new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,

    // Predicate input commitment and memo
    pub predicate_comm: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    pub local_data_comm: <C::LocalDataCommitment as CommitmentScheme>::Output,
    pub memo: [u8; 32],

    pub binding_signature: BindingSignature,
}

impl<C: DelegablePaymentDPCComponents> ToConstraintField<C::InnerField> for InnerCircuitVerifier<C>
where
    <C::AddressCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AddressCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::RecordCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::RecordCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::SerialNumberNonce as CRH>::Parameters: ToConstraintField<C::InnerField>,

    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::Signature as SignatureScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::Signature as SignatureScheme>::PublicKey: ToConstraintField<C::InnerField>,

    MerkleTreeParams<C::MerkleParameters>: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = Vec::new();

        v.extend_from_slice(&self.comm_crh_sig_pp.addr_comm_pp.parameters().to_field_elements()?);
        v.extend_from_slice(&self.comm_crh_sig_pp.rec_comm_pp.parameters().to_field_elements()?);
        v.extend_from_slice(
            &self
                .comm_crh_sig_pp
                .local_data_comm_pp
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(&self.comm_crh_sig_pp.pred_vk_comm_pp.parameters().to_field_elements()?);

        v.extend_from_slice(&self.comm_crh_sig_pp.sn_nonce_crh_pp.parameters().to_field_elements()?);

        v.extend_from_slice(&self.comm_crh_sig_pp.sig_pp.to_field_elements()?);

        v.extend_from_slice(&self.ledger_pp.parameters().to_field_elements()?);
        v.extend_from_slice(&self.ledger_digest.to_field_elements()?);

        for sn in &self.old_serial_numbers {
            v.extend_from_slice(&sn.to_field_elements()?);
        }

        for cm in &self.new_commitments {
            v.extend_from_slice(&cm.to_field_elements()?);
        }

        v.extend_from_slice(&self.predicate_comm.to_field_elements()?);
        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(
            self.memo.as_ref(),
        )?);
        v.extend_from_slice(&self.local_data_comm.to_field_elements()?);

        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(
            &self.binding_signature.to_bytes()[..],
        )?);

        Ok(v)
    }
}
