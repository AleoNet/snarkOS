use crate::dpc::base_dpc::{parameters::CircuitParameters, BaseDPCComponents};
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerkleTreeDigest};
use snarkos_errors::curves::ConstraintFieldError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};

pub struct InnerCircuitVerifierInput<C: BaseDPCComponents> {
    // Commitment, CRH, and signature parameters
    pub circuit_parameters: CircuitParameters<C>,

    // Ledger parameters and digest
    pub ledger_parameters: C::MerkleParameters,
    pub ledger_digest: MerkleTreeDigest<C::MerkleParameters>,

    // Input record serial numbers and death predicate commitments
    pub old_serial_numbers: Vec<<C::Signature as SignatureScheme>::PublicKey>,

    // Output record commitments and birth predicate commitments
    pub new_commitments: Vec<<C::RecordCommitment as CommitmentScheme>::Output>,

    // Predicate input commitment and memo
    pub predicate_commitment: <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
    pub local_data_commitment: <C::LocalDataCommitment as CommitmentScheme>::Output,
    pub memo: [u8; 32],

    pub value_balance: i64,
}

impl<C: BaseDPCComponents> ToConstraintField<C::InnerField> for InnerCircuitVerifierInput<C>
where
    <C::AccountCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::RecordCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::RecordCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::SerialNumberNonce as CRH>::Parameters: ToConstraintField<C::InnerField>,

    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::LocalDataCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::Signature as SignatureScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::Signature as SignatureScheme>::PublicKey: ToConstraintField<C::InnerField>,

    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::InnerField>, ConstraintFieldError> {
        let mut v = Vec::new();

        v.extend_from_slice(
            &self
                .circuit_parameters
                .account_commitment
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .circuit_parameters
                .record_commitment_parameters
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .circuit_parameters
                .local_data_commitment_parameters
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .circuit_parameters
                .predicate_verification_key_commitment_parameters
                .parameters()
                .to_field_elements()?,
        );

        v.extend_from_slice(
            &self
                .circuit_parameters
                .serial_number_nonce_parameters
                .parameters()
                .to_field_elements()?,
        );

        v.extend_from_slice(
            &self
                .circuit_parameters
                .signature_parameters
                .parameters()
                .to_field_elements()?,
        );

        v.extend_from_slice(
            &self
                .circuit_parameters
                .value_commitment_parameters
                .parameters()
                .to_field_elements()?,
        );

        v.extend_from_slice(&self.ledger_parameters.parameters().to_field_elements()?);
        v.extend_from_slice(&self.ledger_digest.to_field_elements()?);

        for sn in &self.old_serial_numbers {
            v.extend_from_slice(&sn.to_field_elements()?);
        }

        for cm in &self.new_commitments {
            v.extend_from_slice(&cm.to_field_elements()?);
        }

        v.extend_from_slice(&self.predicate_commitment.to_field_elements()?);
        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(&self.memo)?);
        v.extend_from_slice(&self.local_data_commitment.to_field_elements()?);

        let value_balance_as_u64 = self.value_balance.abs() as u64;

        let is_negative: bool = self.value_balance.is_negative();

        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(
            &value_balance_as_u64.to_le_bytes()[..],
        )?);

        v.extend_from_slice(&ToConstraintField::<C::InnerField>::to_field_elements(
            &[is_negative as u8][..],
        )?);

        Ok(v)
    }
}
