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
    algorithms::{CommitmentScheme, SignatureScheme},
    dpc::DPCComponents,
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

    // Predicate Commitment
    predicate_commitment: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_randomness: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,

    // Local data commitments, witnesses, and digest
    local_data_commitment_leaves: Option<Vec<<C::LocalDataCommitment as CommitmentScheme>::Output>>,
    local_data_commitment_leaves_randomness: Option<Vec<<C::LocalDataCommitment as CommitmentScheme>::Randomness>>,
    local_data_witnesses: Option<Vec<MerklePath<<C as DPCComponents>::LocalDataMerkleParameters>>>,
    local_data_commitment: Option<MerkleTreeDigest<<C as DPCComponents>::LocalDataMerkleParameters>>,

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

        let memo = [0u8; 32];

        let predicate_commitment = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default();
        let predicate_randomness = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default();

        // Number of leaves are the number of input + output records
        let num_leaves = num_input_records + num_output_records;
        let local_data_commitment_leaves =
            vec![<C::LocalDataCommitment as CommitmentScheme>::Output::default(); num_leaves];
        let local_data_commitment_leaves_randomness =
            vec![<C::LocalDataCommitment as CommitmentScheme>::Randomness::default(); num_leaves];
        let local_data_witnesses = vec![MerklePath::default(); num_leaves];
        let local_data_commitment = MerkleTreeDigest::<<C as DPCComponents>::LocalDataMerkleParameters>::default();

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

            // Predicate commitment
            predicate_commitment: Some(predicate_commitment),
            predicate_randomness: Some(predicate_randomness),

            // Local data commitments, witnesses, and digest
            local_data_commitment_leaves: Some(local_data_commitment_leaves),
            local_data_commitment_leaves_randomness: Some(local_data_commitment_leaves_randomness),
            local_data_witnesses: Some(local_data_witnesses),
            local_data_commitment: Some(local_data_commitment),

            // Other stuff
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

        // Other stuff
        predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

        // Local data commitments, witnesses, and digest
        local_data_commitment_leaves: &[<C::LocalDataCommitment as CommitmentScheme>::Output],
        local_data_commitment_leaves_randomness: &[<C::LocalDataCommitment as CommitmentScheme>::Randomness],
        local_data_witnesses: &[MerklePath<<C as DPCComponents>::LocalDataMerkleParameters>],
        local_data_commitment: &MerkleTreeDigest<<C as DPCComponents>::LocalDataMerkleParameters>,

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

        let num_leaves = num_input_records + num_output_records;
        assert_eq!(num_leaves, local_data_commitment_leaves.len());
        assert_eq!(num_leaves, local_data_commitment_leaves_randomness.len());
        assert_eq!(num_leaves, local_data_witnesses.len());

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

            // Predicate Commitment
            predicate_commitment: Some(predicate_commitment.clone()),
            predicate_randomness: Some(predicate_randomness.clone()),

            // Local data commitments, witnesses, and digest
            local_data_commitment_leaves: Some(local_data_commitment_leaves.to_vec()),
            local_data_commitment_leaves_randomness: Some(local_data_commitment_leaves_randomness.to_vec()),
            local_data_witnesses: Some(local_data_witnesses.to_vec()),
            local_data_commitment: Some(local_data_commitment.clone()),

            // Other stuff
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
            // Predicate commitment
            self.predicate_commitment.get()?,
            self.predicate_randomness.get()?,
            // Local data commitments, witnesses, and digest
            self.local_data_commitment_leaves.get()?,
            self.local_data_commitment_leaves_randomness.get()?,
            self.local_data_witnesses.get()?,
            self.local_data_commitment.get()?,
            // Other stuff
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
