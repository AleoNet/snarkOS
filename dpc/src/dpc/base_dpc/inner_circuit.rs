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
use snarkos_models::gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use snarkos_objects::AccountPrivateKey;
use snarkvm_errors::gadgets::SynthesisError;
use snarkvm_models::algorithms::{CommitmentScheme, SignatureScheme};

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
    old_serial_numbers: Option<Vec<<C::Signature as SignatureScheme>::PublicKey>>,

    // Inputs for new records.
    new_records: Option<Vec<DPCRecord<C>>>,
    new_serial_number_nonce_randomness: Option<Vec<[u8; 32]>>,
    new_commitments: Option<Vec<<C::RecordCommitment as CommitmentScheme>::Output>>,

    // Commitment to Predicates and to local data.
    predicate_commitment: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_randomness: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,

    local_data_commitment: Option<<C::LocalDataCommitment as CommitmentScheme>::Output>,
    local_data_randomness: Option<<C::LocalDataCommitment as CommitmentScheme>::Randomness>,

    memo: Option<[u8; 32]>,
    auxiliary: Option<[u8; 32]>,

    input_value_commitments: Option<Vec<[u8; 32]>>,
    output_value_commitments: Option<Vec<[u8; 32]>>,
    value_balance: Option<i64>,
    binding_signature: Option<BindingSignature>,
}

impl<C: BaseDPCComponents> InnerCircuit<C> {
    pub fn blank(circuit_parameters: &CircuitParameters<C>, ledger_parameters: &C::MerkleParameters) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;
        let digest = MerkleTreeDigest::<C::MerkleParameters>::default();

        let old_serial_numbers = vec![<C::Signature as SignatureScheme>::PublicKey::default(); num_input_records];
        let old_records = vec![DPCRecord::default(); num_input_records];
        let old_witnesses = vec![MerklePath::default(); num_input_records];
        let old_account_private_keys = vec![AccountPrivateKey::default(); num_input_records];

        let new_commitments = vec![<C::RecordCommitment as CommitmentScheme>::Output::default(); num_output_records];
        let new_serial_number_nonce_randomness = vec![[0u8; 32]; num_output_records];
        let new_records = vec![DPCRecord::default(); num_output_records];

        let auxiliary = [1u8; 32];
        let memo = [0u8; 32];

        let predicate_commitment = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default();
        let predicate_randomness = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default();

        let local_data_commitment = <C::LocalDataCommitment as CommitmentScheme>::Output::default();
        let local_data_randomness = <C::LocalDataCommitment as CommitmentScheme>::Randomness::default();

        let input_value_commitments = vec![[0u8; 32]; num_input_records];
        let output_value_commitments = vec![[0u8; 32]; num_output_records];
        let value_balance: i64 = 0;
        let binding_signature = BindingSignature::default();

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

            // Other stuff
            predicate_commitment: Some(predicate_commitment),
            predicate_randomness: Some(predicate_randomness),
            local_data_commitment: Some(local_data_commitment),
            local_data_randomness: Some(local_data_randomness),
            memo: Some(memo),
            auxiliary: Some(auxiliary),

            input_value_commitments: Some(input_value_commitments),
            output_value_commitments: Some(output_value_commitments),
            value_balance: Some(value_balance),
            binding_signature: Some(binding_signature),
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
        old_serial_numbers: &[<C::Signature as SignatureScheme>::PublicKey],

        // New records
        new_records: &[DPCRecord<C>],
        new_serial_number_nonce_randomness: &[[u8; 32]],
        new_commitments: &[<C::RecordCommitment as CommitmentScheme>::Output],

        // Other stuff
        predicate_commitment: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_randomness: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

        local_data_commitment: &<C::LocalDataCommitment as CommitmentScheme>::Output,
        local_data_randomness: &<C::LocalDataCommitment as CommitmentScheme>::Randomness,

        memo: &[u8; 32],
        auxiliary: &[u8; 32],

        input_value_commitments: &[[u8; 32]],
        output_value_commitments: &[[u8; 32]],
        value_balance: i64,
        binding_signature: &BindingSignature,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_records.len());
        assert_eq!(num_input_records, old_witnesses.len());
        assert_eq!(num_input_records, old_account_private_keys.len());
        assert_eq!(num_input_records, old_serial_numbers.len());

        assert_eq!(num_output_records, new_records.len());
        assert_eq!(num_output_records, new_serial_number_nonce_randomness.len());
        assert_eq!(num_output_records, new_commitments.len());

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

            // Other stuff
            predicate_commitment: Some(predicate_commitment.clone()),
            predicate_randomness: Some(predicate_randomness.clone()),

            local_data_commitment: Some(local_data_commitment.clone()),
            local_data_randomness: Some(local_data_randomness.clone()),

            memo: Some(memo.clone()),
            auxiliary: Some(auxiliary.clone()),

            input_value_commitments: Some(input_value_commitments.to_vec()),
            output_value_commitments: Some(output_value_commitments.to_vec()),
            value_balance: Some(value_balance),
            binding_signature: Some(binding_signature.clone()),
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
            // Other stuff
            self.predicate_commitment.get()?,
            self.predicate_randomness.get()?,
            self.local_data_commitment.get()?,
            self.local_data_randomness.get()?,
            self.memo.get()?,
            self.auxiliary.get()?,
            self.input_value_commitments.get()?,
            self.output_value_commitments.get()?,
            *self.value_balance.get()?,
            self.binding_signature.get()?,
        )?;
        Ok(())
    }
}
