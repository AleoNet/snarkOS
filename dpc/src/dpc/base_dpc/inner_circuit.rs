use crate::{
    dpc::base_dpc::{
        binding_signature::BindingSignature,
        inner_circuit_gadget::execute_inner_proof_gadget,
        parameters::CircuitParameters,
        record::DPCRecord,
        BaseDPCComponents,
    },
    Assignment,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, SignatureScheme, CRH},
    curves::ModelParameters,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};
use snarkos_objects::AccountPrivateKey;

#[derive(Derivative)]
#[derivative(Clone(bound = "C: BaseDPCComponents"))]
pub struct InnerCircuit<C: BaseDPCComponents> {
    // Parameters
    circuit_parameters: Option<CircuitParameters<C>>,
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
    new_records_field_elements: Option<Vec<Vec<<C::EncryptionModelParameters as ModelParameters>::BaseField>>>,
    new_records_group_encoding: Option<
        Vec<
            Vec<(
                <C::EncryptionModelParameters as ModelParameters>::BaseField,
                <C::EncryptionModelParameters as ModelParameters>::BaseField,
            )>,
        >,
    >,
    new_records_encryption_randomness: Option<Vec<<C::AccountEncryption as EncryptionScheme>::Randomness>>,
    new_records_encryption_blinding_exponents:
        Option<Vec<Vec<<C::AccountEncryption as EncryptionScheme>::BlindingExponent>>>,
    new_records_ciphertext_and_fq_high_selectors: Option<Vec<(Vec<bool>, Vec<bool>)>>,
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
    pub fn blank(circuit_parameters: &CircuitParameters<C>, ledger_parameters: &C::MerkleParameters) -> Self {
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

        // TODO (raychu86) Fix the lengths to be generic
        let record_encoding_length = 7;
        let base_field_default = <C::EncryptionModelParameters as ModelParameters>::BaseField::default();
        let new_records_field_elements = vec![vec![base_field_default; record_encoding_length]; num_output_records];
        let new_records_group_encoding =
            vec![vec![(base_field_default, base_field_default); record_encoding_length]; num_output_records];

        let new_records_encryption_randomness =
            vec![<C::AccountEncryption as EncryptionScheme>::Randomness::default(); num_output_records];
        let new_records_encryption_blinding_exponents = vec![
                vec![<C::AccountEncryption as EncryptionScheme>::BlindingExponent::default(); record_encoding_length];
                num_output_records
            ];
        let new_records_ciphertext_and_fq_high_selectors = vec![
            (vec![false; record_encoding_length + 1], vec![
                false;
                record_encoding_length
            ]);
            num_output_records
        ];

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
            circuit_parameters: Some(circuit_parameters.clone()),
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
            new_records_field_elements: Some(new_records_field_elements),
            new_records_group_encoding: Some(new_records_group_encoding),

            new_records_encryption_randomness: Some(new_records_encryption_randomness),
            new_records_encryption_blinding_exponents: Some(new_records_encryption_blinding_exponents),
            new_records_ciphertext_and_fq_high_selectors: Some(new_records_ciphertext_and_fq_high_selectors),
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
        circuit_parameters: &CircuitParameters<C>,
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
        new_records_field_elements: &[Vec<<C::EncryptionModelParameters as ModelParameters>::BaseField>],
        new_records_group_encoding: &[Vec<(
            <C::EncryptionModelParameters as ModelParameters>::BaseField,
            <C::EncryptionModelParameters as ModelParameters>::BaseField,
        )>],
        new_records_encryption_randomness: &[<C::AccountEncryption as EncryptionScheme>::Randomness],
        new_records_encryption_blinding_exponents: &[Vec<
            <C::AccountEncryption as EncryptionScheme>::BlindingExponent,
        >],
        new_records_ciphertext_and_fq_high_selectors: &[(Vec<bool>, Vec<bool>)],
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
        assert_eq!(num_output_records, new_records_field_elements.len());
        assert_eq!(num_output_records, new_records_group_encoding.len());
        assert_eq!(num_output_records, new_records_encryption_randomness.len());
        assert_eq!(num_output_records, new_records_encryption_blinding_exponents.len());
        assert_eq!(num_output_records, new_records_ciphertext_and_fq_high_selectors.len());
        assert_eq!(num_output_records, new_records_ciphertext_hashes.len());

        let record_encoding_length = 7;

        for field_elements in new_records_field_elements {
            assert_eq!(field_elements.len(), record_encoding_length);
        }

        for group_encoding in new_records_group_encoding {
            assert_eq!(group_encoding.len(), record_encoding_length);
        }

        for blinding_exponents in new_records_encryption_blinding_exponents {
            assert_eq!(blinding_exponents.len(), record_encoding_length);
        }

        for (ciphertext_selectors, fq_high_selectors) in new_records_ciphertext_and_fq_high_selectors {
            assert_eq!(ciphertext_selectors.len(), record_encoding_length);
            assert_eq!(fq_high_selectors.len(), record_encoding_length + 1);
        }

        Self {
            // Parameters
            circuit_parameters: Some(circuit_parameters.clone()),
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
            new_records_field_elements: Some(new_records_field_elements.to_vec()),
            new_records_group_encoding: Some(new_records_group_encoding.to_vec()),
            new_records_encryption_randomness: Some(new_records_encryption_randomness.to_vec()),
            new_records_encryption_blinding_exponents: Some(new_records_encryption_blinding_exponents.to_vec()),
            new_records_ciphertext_and_fq_high_selectors: Some(new_records_ciphertext_and_fq_high_selectors.to_vec()),
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
            self.circuit_parameters.get()?,
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
            self.new_records_field_elements.get()?,
            self.new_records_group_encoding.get()?,
            self.new_records_encryption_randomness.get()?,
            self.new_records_encryption_blinding_exponents.get()?,
            self.new_records_ciphertext_and_fq_high_selectors.get()?,
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
