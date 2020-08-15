use crate::{
    base_dpc::{
        outer_circuit_gadget::execute_outer_proof_gadget,
        parameters::SystemParameters,
        program::PrivateProgramInput,
        BaseDPCComponents,
    },
    Assignment,
};
use snarkos_algorithms::merkle_tree::MerkleTreeDigest;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, MerkleParameters, SignatureScheme, CRH, SNARK},
    curves::to_field_vec::ToConstraintField,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};
use snarkos_objects::AleoAmount;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct OuterCircuit<C: BaseDPCComponents> {
    system_parameters: Option<SystemParameters<C>>,

    // Inner snark verifier public inputs
    ledger_parameters: Option<C::MerkleParameters>,
    ledger_digest: Option<MerkleTreeDigest<C::MerkleParameters>>,
    old_serial_numbers: Option<Vec<<C::AccountSignature as SignatureScheme>::PublicKey>>,
    new_commitments: Option<Vec<<C::RecordCommitment as CommitmentScheme>::Output>>,
    new_encrypted_record_hashes: Option<Vec<<C::EncryptedRecordCRH as CRH>::Output>>,
    memo: Option<[u8; 32]>,
    value_balance: Option<AleoAmount>,
    network_id: Option<u8>,

    // Inner snark verifier private inputs
    inner_snark_vk: Option<<C::InnerSNARK as SNARK>::VerificationParameters>,
    inner_snark_proof: Option<<C::InnerSNARK as SNARK>::Proof>,

    old_private_program_inputs: Option<Vec<PrivateProgramInput>>,
    new_private_program_inputs: Option<Vec<PrivateProgramInput>>,

    program_commitment: Option<<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output>,
    program_randomness: Option<<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness>,
    local_data_root: Option<<C::LocalDataCRH as CRH>::Output>,

    inner_snark_id: Option<<C::InnerSNARKVerificationKeyCRH as CRH>::Output>,
}

impl<C: BaseDPCComponents> OuterCircuit<C> {
    pub fn blank(
        system_parameters: &SystemParameters<C>,
        ledger_parameters: &C::MerkleParameters,
        inner_snark_vk: &<C::InnerSNARK as SNARK>::VerificationParameters,
        inner_snark_proof: &<C::InnerSNARK as SNARK>::Proof,
        program_snark_vk_and_proof: &PrivateProgramInput,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        let ledger_digest = Some(MerkleTreeDigest::<C::MerkleParameters>::default());
        let old_serial_numbers = Some(vec![
            <C::AccountSignature as SignatureScheme>::PublicKey::default();
            num_input_records
        ]);
        let new_commitments = Some(vec![
            <C::RecordCommitment as CommitmentScheme>::Output::default();
            num_output_records
        ]);
        let new_encrypted_record_hashes = Some(vec![
            <C::EncryptedRecordCRH as CRH>::Output::default();
            num_output_records
        ]);
        let memo = Some([0u8; 32]);
        let value_balance = Some(AleoAmount::ZERO);
        let network_id = Some(0);

        let old_private_program_inputs = Some(vec![program_snark_vk_and_proof.clone(); num_input_records]);
        let new_private_program_inputs = Some(vec![program_snark_vk_and_proof.clone(); num_output_records]);

        let program_commitment = Some(<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output::default());
        let program_randomness = Some(<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness::default());
        let local_data_root = Some(<C::LocalDataCRH as CRH>::Output::default());

        let inner_snark_id = Some(<C::InnerSNARKVerificationKeyCRH as CRH>::Output::default());

        Self {
            system_parameters: Some(system_parameters.clone()),

            ledger_parameters: Some(ledger_parameters.clone()),
            ledger_digest,
            old_serial_numbers,
            new_commitments,
            memo,
            new_encrypted_record_hashes,
            value_balance,
            network_id,

            inner_snark_vk: Some(inner_snark_vk.clone()),
            inner_snark_proof: Some(inner_snark_proof.clone()),

            old_private_program_inputs,
            new_private_program_inputs,

            program_commitment,
            program_randomness,
            local_data_root,

            inner_snark_id,
        }
    }

    pub fn new(
        system_parameters: &SystemParameters<C>,

        // Inner SNARK public inputs
        ledger_parameters: &C::MerkleParameters,
        ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,
        old_serial_numbers: &Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
        new_commitments: &Vec<<C::RecordCommitment as CommitmentScheme>::Output>,
        new_encrypted_record_hashes: &[<C::EncryptedRecordCRH as CRH>::Output],
        memo: &[u8; 32],
        value_balance: AleoAmount,
        network_id: u8,

        // Inner SNARK private inputs
        inner_snark_vk: &<C::InnerSNARK as SNARK>::VerificationParameters,
        inner_snark_proof: &<C::InnerSNARK as SNARK>::Proof,

        // Private program input = Verification key and input
        // Commitment contains commitment to hash of death program vk.
        old_private_program_inputs: &[PrivateProgramInput],

        // Private program input = Verification key and input
        // Commitment contains commitment to hash of birth program vk.
        new_private_program_inputs: &[PrivateProgramInput],

        program_commitment: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Output,
        program_randomness: &<C::ProgramVerificationKeyCommitment as CommitmentScheme>::Randomness,
        local_data_root: &<C::LocalDataCRH as CRH>::Output,

        // Inner SNARK ID
        inner_snark_id: &<C::InnerSNARKVerificationKeyCRH as CRH>::Output,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_private_program_inputs.len());
        assert_eq!(num_output_records, new_private_program_inputs.len());
        assert_eq!(num_output_records, new_commitments.len());
        assert_eq!(num_output_records, new_encrypted_record_hashes.len());

        Self {
            system_parameters: Some(system_parameters.clone()),

            ledger_parameters: Some(ledger_parameters.clone()),
            ledger_digest: Some(ledger_digest.clone()),
            old_serial_numbers: Some(old_serial_numbers.to_vec()),
            new_commitments: Some(new_commitments.to_vec()),
            new_encrypted_record_hashes: Some(new_encrypted_record_hashes.to_vec()),
            memo: Some(memo.clone()),
            value_balance: Some(value_balance),
            network_id: Some(network_id),

            inner_snark_vk: Some(inner_snark_vk.clone()),
            inner_snark_proof: Some(inner_snark_proof.clone()),

            old_private_program_inputs: Some(old_private_program_inputs.to_vec()),
            new_private_program_inputs: Some(new_private_program_inputs.to_vec()),

            program_commitment: Some(program_commitment.clone()),
            program_randomness: Some(program_randomness.clone()),
            local_data_root: Some(local_data_root.clone()),

            inner_snark_id: Some(inner_snark_id.clone()),
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::OuterField> for OuterCircuit<C>
where
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

    <<C::MerkleParameters as MerkleParameters>::H as CRH>::Parameters: ToConstraintField<C::InnerField>,
    MerkleTreeDigest<C::MerkleParameters>: ToConstraintField<C::InnerField>,
{
    fn generate_constraints<CS: ConstraintSystem<C::OuterField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_outer_proof_gadget::<C, CS>(
            cs,
            self.system_parameters.get()?,
            self.ledger_parameters.get()?,
            self.ledger_digest.get()?,
            self.old_serial_numbers.get()?,
            self.new_commitments.get()?,
            self.new_encrypted_record_hashes.get()?,
            self.memo.get()?,
            *self.value_balance.get()?,
            *self.network_id.get()?,
            self.inner_snark_vk.get()?,
            self.inner_snark_proof.get()?,
            self.old_private_program_inputs.get()?.as_slice(),
            self.new_private_program_inputs.get()?.as_slice(),
            self.program_commitment.get()?,
            self.program_randomness.get()?,
            self.local_data_root.get()?,
            self.inner_snark_id.get()?,
        )?;
        Ok(())
    }
}
