use crate::{
    dpc::base_dpc::{
        binding_signature::BindingSignature,
        inner_circuit_gadget::execute_inner_proof_gadget,
        parameters::SystemParameters,
        record::DPCRecord,
        record_encryption::RecordEncryptionGadgetComponents,
        BaseDPCComponents,
    },
    Assignment,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme, CRH},
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};
use snarkos_objects::AccountPrivateKey;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct InnerCircuit<C: BaseDPCComponents> {
    // Parameters
    system_parameters: Option<SystemParameters<C>>,
    ledger_parameters: Option<C::MerkleParameters>,

    ledger_digest: Option<MerkleTreeDigest<C::MerkleParameters>>,

    // Inputs for old records.
    old_records: Option<Vec<DPCRecord<C>>>,
    old_witnesses: Option<Vec<MerklePath<C::MerkleParameters>>>,
    old_account_private_keys: Option<Vec<AccountPrivateKey<C>>>,
    old_serial_numbers: Option<Vec<<C::AccountSignature as SignatureScheme>::PublicKey>>,

    // Inputs for new records.
    new_records: Option<Vec<DPCRecord<C>>>,
    new_serial_number_nonce_randomness: Option<Vec<[u8; 32]>>,
    new_commitments: Option<Vec<<C::RecordCommitment as CommitmentScheme>::Output>>,

    // Inputs for encryption of new records.
    new_records_encryption_randomness: Option<Vec<<C::AccountEncryption as EncryptionScheme>::Randomness>>,
    new_records_encryption_gadget_components: Option<Vec<RecordEncryptionGadgetComponents<C>>>,
    new_records_ciphertext_hashes: Option<Vec<<C::RecordCiphertextCRH as CRH>::Output>>,

    // Commitment to Predicates and to local data.
    predicate_commitment: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_randomness: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,

    local_data_commitment: Option<<C::LocalDataCRH as CRH>::Output>,
    local_data_commitment_randomizers: Option<Vec<<C::LocalDataCommitment as CommitmentScheme>::Randomness>>,

    memo: Option<[u8; 32]>,

    input_value_commitments: Option<Vec<<C::ValueCommitment as CommitmentScheme>::Output>>,
    input_value_commitment_randomness: Option<Vec<<C::ValueCommitment as CommitmentScheme>::Randomness>>,
    output_value_commitments: Option<Vec<<C::ValueCommitment as CommitmentScheme>::Output>>,
    output_value_commitment_randomness: Option<Vec<<C::ValueCommitment as CommitmentScheme>::Randomness>>,
    value_balance: Option<i64>,
    binding_signature: Option<BindingSignature>,

    network_id: Option<u8>,
}

impl<C: BaseDPCComponents> InnerCircuit<C> {
    pub fn blank(system_parameters: &SystemParameters<C>, ledger_parameters: &C::MerkleParameters) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;
        let digest = MerkleTreeDigest::<C::MerkleParameters>::default();

        let old_serial_numbers =
            vec![<C::AccountSignature as SignatureScheme>::PublicKey::default(); num_input_records];
        let old_records = vec![DPCRecord::default(); num_input_records];
        let old_witnesses = vec![MerklePath::default(); num_input_records];
        let old_account_private_keys = vec![AccountPrivateKey::default(); num_input_records];

        let new_commitments = vec![<C::RecordCommitment as CommitmentScheme>::Output::default(); num_output_records];
        let new_serial_number_nonce_randomness = vec![[0u8; 32]; num_output_records];
        let new_records = vec![DPCRecord::default(); num_output_records];

        let new_records_encryption_randomness =
            vec![<C::AccountEncryption as EncryptionScheme>::Randomness::default(); num_output_records];

        let new_records_encryption_gadget_components =
            vec![RecordEncryptionGadgetComponents::<C>::default(); num_output_records];

        let new_records_ciphertext_hashes =
            vec![<C::RecordCiphertextCRH as CRH>::Output::default(); num_output_records];

        let memo = [0u8; 32];

        let predicate_commitment = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default();
        let predicate_randomness = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default();

        let local_data_commitment = <C::LocalDataCRH as CRH>::Output::default();
        let local_data_commitment_randomizers = vec![
            <C::LocalDataCommitment as CommitmentScheme>::Randomness::default();
            num_input_records + num_output_records
        ];

        let input_value_commitments =
            vec![<C::ValueCommitment as CommitmentScheme>::Output::default(); num_input_records];
        let input_value_commitment_randomness =
            vec![<C::ValueCommitment as CommitmentScheme>::Randomness::default(); num_input_records];
        let output_value_commitments =
            vec![<C::ValueCommitment as CommitmentScheme>::Output::default(); num_output_records];
        let output_value_commitment_randomness =
            vec![<C::ValueCommitment as CommitmentScheme>::Randomness::default(); num_output_records];
        let value_balance: i64 = 0;
        let binding_signature = BindingSignature::default();

        let network_id: u8 = 0;

        Self {
            // Parameters
            system_parameters: Some(system_parameters.clone()),
            ledger_parameters: Some(ledger_parameters.clone()),

            // Digest
            ledger_digest: Some(digest),

            // Input records
            old_records: Some(old_records),
            old_witnesses: Some(old_witnesses),
            old_account_private_keys: Some(old_account_private_keys),
            old_serial_numbers: Some(old_serial_numbers),

            // Output records
            new_records: Some(new_records),
            new_serial_number_nonce_randomness: Some(new_serial_number_nonce_randomness),
            new_commitments: Some(new_commitments),

            new_records_encryption_randomness: Some(new_records_encryption_randomness),
            new_records_encryption_gadget_components: Some(new_records_encryption_gadget_components),
            new_records_ciphertext_hashes: Some(new_records_ciphertext_hashes),

            // Other stuff
            predicate_commitment: Some(predicate_commitment),
            predicate_randomness: Some(predicate_randomness),
            local_data_commitment: Some(local_data_commitment),
            local_data_commitment_randomizers: Some(local_data_commitment_randomizers),
            memo: Some(memo),

            input_value_commitments: Some(input_value_commitments),
            input_value_commitment_randomness: Some(input_value_commitment_randomness),
            output_value_commitments: Some(output_value_commitments),
            output_value_commitment_randomness: Some(output_value_commitment_randomness),
            value_balance: Some(value_balance),
            binding_signature: Some(binding_signature),

            network_id: Some(network_id),
        }
    }

    pub fn new(
        // Parameters
        system_parameters: &SystemParameters<C>,
        ledger_parameters: &C::MerkleParameters,

        // Digest
        ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

        // Old records
        old_records: &[DPCRecord<C>],
        old_witnesses: &[MerklePath<C::MerkleParameters>],
        old_account_private_keys: &[AccountPrivateKey<C>],
        old_serial_numbers: &[<C::AccountSignature as SignatureScheme>::PublicKey],

        // New records
        new_records: &[DPCRecord<C>],
        new_serial_number_nonce_randomness: &[[u8; 32]],
        new_commitments: &[<C::RecordCommitment as CommitmentScheme>::Output],

        new_records_encryption_randomness: &[<C::AccountEncryption as EncryptionScheme>::Randomness],
        new_records_encryption_gadget_components: &[RecordEncryptionGadgetComponents<C>],
        new_records_ciphertext_hashes: &[<C::RecordCiphertextCRH as CRH>::Output],

        // Other stuff
        predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

        local_data_commitment: &<C::LocalDataCRH as CRH>::Output,
        local_data_commitment_randomizers: &[<C::LocalDataCommitment as CommitmentScheme>::Randomness],

        memo: &[u8; 32],

        input_value_commitments: &[<C::ValueCommitment as CommitmentScheme>::Output],
        input_value_commitment_randomness: &[<C::ValueCommitment as CommitmentScheme>::Randomness],
        output_value_commitments: &[<C::ValueCommitment as CommitmentScheme>::Output],
        output_value_commitment_randomness: &[<C::ValueCommitment as CommitmentScheme>::Randomness],
        value_balance: i64,
        binding_signature: &BindingSignature,

        network_id: u8,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_records.len());
        assert_eq!(num_input_records, old_witnesses.len());
        assert_eq!(num_input_records, old_account_private_keys.len());
        assert_eq!(num_input_records, old_serial_numbers.len());
        assert_eq!(num_input_records, input_value_commitments.len());
        assert_eq!(num_input_records, input_value_commitment_randomness.len());

        assert_eq!(num_output_records, new_records.len());
        assert_eq!(num_output_records, new_serial_number_nonce_randomness.len());
        assert_eq!(num_output_records, new_commitments.len());
        assert_eq!(num_output_records, output_value_commitments.len());
        assert_eq!(num_output_records, output_value_commitment_randomness.len());

        assert_eq!(num_output_records, new_records_encryption_randomness.len());
        assert_eq!(num_output_records, new_records_encryption_gadget_components.len());
        assert_eq!(num_output_records, new_records_ciphertext_hashes.len());

        // TODO (raychu86) Fix the lengths to be generic
        let record_encoding_length = 7;

        for gadget_components in new_records_encryption_gadget_components {
            assert_eq!(gadget_components.record_field_elements.len(), record_encoding_length);
            assert_eq!(gadget_components.record_group_encoding.len(), record_encoding_length);
            assert_eq!(gadget_components.ciphertext_selectors.len(), record_encoding_length + 1);
            assert_eq!(gadget_components.fq_high_selectors.len(), record_encoding_length);
            assert_eq!(
                gadget_components.encryption_blinding_exponents.len(),
                record_encoding_length
            );
        }

        Self {
            // Parameters
            system_parameters: Some(system_parameters.clone()),
            ledger_parameters: Some(ledger_parameters.clone()),

            // Digest
            ledger_digest: Some(ledger_digest.clone()),

            // Input records
            old_records: Some(old_records.to_vec()),
            old_witnesses: Some(old_witnesses.to_vec()),
            old_account_private_keys: Some(old_account_private_keys.to_vec()),
            old_serial_numbers: Some(old_serial_numbers.to_vec()),

            // Output records
            new_records: Some(new_records.to_vec()),
            new_serial_number_nonce_randomness: Some(new_serial_number_nonce_randomness.to_vec()),
            new_commitments: Some(new_commitments.to_vec()),

            new_records_encryption_randomness: Some(new_records_encryption_randomness.to_vec()),
            new_records_encryption_gadget_components: Some(new_records_encryption_gadget_components.to_vec()),
            new_records_ciphertext_hashes: Some(new_records_ciphertext_hashes.to_vec()),

            // Other stuff
            predicate_commitment: Some(predicate_commitment.clone()),
            predicate_randomness: Some(predicate_randomness.clone()),

            local_data_commitment: Some(local_data_commitment.clone()),
            local_data_commitment_randomizers: Some(local_data_commitment_randomizers.to_vec()),

            memo: Some(memo.clone()),

            input_value_commitments: Some(input_value_commitments.to_vec()),
            input_value_commitment_randomness: Some(input_value_commitment_randomness.to_vec()),
            output_value_commitments: Some(output_value_commitments.to_vec()),
            output_value_commitment_randomness: Some(output_value_commitment_randomness.to_vec()),
            value_balance: Some(value_balance),
            binding_signature: Some(binding_signature.clone()),

            network_id: Some(network_id),
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for InnerCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_inner_proof_gadget::<C, CS>(
            cs,
            // Parameters
            self.system_parameters.get()?,
            self.ledger_parameters.get()?,
            // Digest
            self.ledger_digest.get()?,
            // Old records
            self.old_records.get()?,
            self.old_witnesses.get()?,
            self.old_account_private_keys.get()?,
            self.old_serial_numbers.get()?,
            // New records
            self.new_records.get()?,
            self.new_serial_number_nonce_randomness.get()?,
            self.new_commitments.get()?,
            self.new_records_encryption_randomness.get()?,
            self.new_records_encryption_gadget_components.get()?,
            self.new_records_ciphertext_hashes.get()?,
            // Other stuff
            self.predicate_commitment.get()?,
            self.predicate_randomness.get()?,
            self.local_data_commitment.get()?,
            self.local_data_commitment_randomizers.get()?,
            self.memo.get()?,
            self.input_value_commitments.get()?,
            self.input_value_commitment_randomness.get()?,
            self.output_value_commitments.get()?,
            self.output_value_commitment_randomness.get()?,
            *self.value_balance.get()?,
            self.binding_signature.get()?,
            *self.network_id.get()?,
        )?;
        Ok(())
    }
}
