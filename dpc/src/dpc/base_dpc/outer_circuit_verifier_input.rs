use crate::dpc::base_dpc::{inner_circuit_verifier_input::InnerCircuitVerifierInput, BaseDPCComponents};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, MerkleParameters, SignatureScheme, CRH},
    curves::to_field_vec::ToConstraintField,
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct OuterCircuitVerifierInput<C: BaseDPCComponents> {
    pub inner_snark_verifier_input: InnerCircuitVerifierInput<C>,
}

impl<C: BaseDPCComponents> ToConstraintField<C::OuterField> for OuterCircuitVerifierInput<C>
where
    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::OuterField>,
    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::OuterField>,
    <C::ProgramVerificationKeyHash as CRH>::Parameters: ToConstraintField<C::OuterField>,

    <C::AccountCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::AccountEncryption as EncryptionScheme>::Parameters: ToConstraintField<C::InnerField>,

    <C::AccountSignature as SignatureScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::AccountSignature as SignatureScheme>::PublicKey: ToConstraintField<C::InnerField>,

    <C::RecordCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::RecordCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::EncryptedRecordCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,
    <C::EncryptedRecordCRH as CRH>::Output: ToConstraintField<C::InnerField>,

    <C::SerialNumberNonceCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,

    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,
    <C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output: ToConstraintField<C::InnerField>,

    <C::LocalDataCRH as CRH>::Parameters: ToConstraintField<C::InnerField>,
    <C::LocalDataCRH as CRH>::Output: ToConstraintField<C::InnerField>,

    <C::ValueCommitment as CommitmentScheme>::Parameters: ToConstraintField<C::InnerField>,

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,
{
    fn to_field_elements(&self) -> Result<Vec<C::OuterField>, ConstraintFieldError> {
        let mut v = Vec::new();

        v.extend_from_slice(
            &self
                .inner_snark_verifier_input
                .system_parameters
                .program_verification_key_commitment
                .parameters()
                .to_field_elements()?,
        );
        v.extend_from_slice(
            &self
                .inner_snark_verifier_input
                .system_parameters
                .program_verification_key_hash
                .parameters()
                .to_field_elements()?,
        );

        // Convert inner snark verifier inputs into `OuterField` field elements

        let inner_snark_field_elements = &self.inner_snark_verifier_input.to_field_elements()?;

        for inner_snark_fe in inner_snark_field_elements {
            let inner_snark_fe_bytes = to_bytes![inner_snark_fe].map_err(|_| SynthesisError::AssignmentMissing)?;
            v.extend_from_slice(&ToConstraintField::<C::OuterField>::to_field_elements(
                inner_snark_fe_bytes.as_slice(),
            )?);
        }

        v.extend_from_slice(&self.inner_snark_verifier_input.program_commitment.to_field_elements()?);
        Ok(v)
    }
}
