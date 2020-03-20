use crate::{
    constraints::{delegable_payment_dpc::execute_core_checks_gadget, Assignment},
    dpc::{
        address::AddressSecretKey,
        delegable_payment_dpc::{
            binding_signature::BindingSignature,
            parameters::CommCRHSigPublicParameters,
            record::DPCRecord,
            DelegablePaymentDPCComponents,
        },
    },
    ledger::MerkleTreeParams,
};
use snarkos_algorithms::merkle_tree::{MerklePath, MerkleTreeDigest};
use snarkos_errors::gadgets::SynthesisError;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme},
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct InnerCircuit<C: DelegablePaymentDPCComponents> {
    // Parameters
    comm_crh_sig_parameters: Option<CommCRHSigPublicParameters<C>>,
    ledger_parameters: Option<MerkleTreeParams<C::MerkleParameters>>,

    ledger_digest: Option<MerkleTreeDigest<C::MerkleParameters>>,

    // Inputs for old records.
    old_records: Option<Vec<DPCRecord<C>>>,
    old_witnesses: Option<Vec<MerklePath<C::MerkleParameters>>>,
    old_address_secret_keys: Option<Vec<AddressSecretKey<C>>>,
    old_serial_numbers: Option<Vec<<C::Signature as SignatureScheme>::PublicKey>>,

    // Inputs for new records.
    new_records: Option<Vec<DPCRecord<C>>>,
    new_sn_nonce_randomness: Option<Vec<[u8; 32]>>,
    new_commitments: Option<Vec<<C::RecordCommitment as CommitmentScheme>::Output>>,

    // Commitment to Predicates and to local data.
    predicate_comm: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output>,
    predicate_rand: Option<<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness>,

    local_data_comm: Option<<C::LocalDataCommitment as CommitmentScheme>::Output>,
    local_data_rand: Option<<C::LocalDataCommitment as CommitmentScheme>::Randomness>,

    memo: Option<[u8; 32]>,
    auxiliary: Option<[u8; 32]>,
    binding_signature: Option<BindingSignature>,
}

impl<C: DelegablePaymentDPCComponents> InnerCircuit<C> {
    pub fn blank(
        comm_and_crh_parameters: &CommCRHSigPublicParameters<C>,
        ledger_parameters: &MerkleTreeParams<C::MerkleParameters>,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;
        let digest = MerkleTreeDigest::<C::MerkleParameters>::default();

        let old_sn = vec![<C::Signature as SignatureScheme>::PublicKey::default(); num_input_records];
        let old_records = vec![DPCRecord::default(); num_input_records];
        let old_witnesses = vec![MerklePath::default(); num_input_records];
        let old_address_secret_keys = vec![AddressSecretKey::default(); num_input_records];

        let new_cm = vec![<C::RecordCommitment as CommitmentScheme>::Output::default(); num_output_records];
        let new_sn_nonce_randomness = vec![[0u8; 32]; num_output_records];
        let new_records = vec![DPCRecord::default(); num_output_records];

        let auxiliary = [1u8; 32];
        let memo = [0u8; 32];

        let predicate_comm = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output::default();
        let predicate_rand = <C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness::default();

        let local_data_comm = <C::LocalDataCommitment as CommitmentScheme>::Output::default();
        let local_data_rand = <C::LocalDataCommitment as CommitmentScheme>::Randomness::default();

        let binding_signature = BindingSignature::default();

        Self {
            // Parameters
            comm_crh_sig_parameters: Some(comm_and_crh_parameters.clone()),
            ledger_parameters: Some(ledger_parameters.clone()),

            // Digest
            ledger_digest: Some(digest),

            // Input records
            old_records: Some(old_records),
            old_witnesses: Some(old_witnesses),
            old_address_secret_keys: Some(old_address_secret_keys),
            old_serial_numbers: Some(old_sn),

            // Output records
            new_records: Some(new_records),
            new_sn_nonce_randomness: Some(new_sn_nonce_randomness),
            new_commitments: Some(new_cm),

            // Other stuff
            predicate_comm: Some(predicate_comm),
            predicate_rand: Some(predicate_rand),
            local_data_comm: Some(local_data_comm),
            local_data_rand: Some(local_data_rand),
            memo: Some(memo),
            auxiliary: Some(auxiliary),
            binding_signature: Some(binding_signature),
        }
    }

    pub fn new(
        // Parameters
        comm_crh_sig_parameters: &CommCRHSigPublicParameters<C>,
        ledger_parameters: &MerkleTreeParams<C::MerkleParameters>,

        // Digest
        ledger_digest: &MerkleTreeDigest<C::MerkleParameters>,

        // Old records
        old_records: &[DPCRecord<C>],
        old_witnesses: &[MerklePath<C::MerkleParameters>],
        old_address_secret_keys: &[AddressSecretKey<C>],
        old_serial_numbers: &[<C::Signature as SignatureScheme>::PublicKey],

        // New records
        new_records: &[DPCRecord<C>],
        new_sn_nonce_randomness: &[[u8; 32]],
        new_commitments: &[<C::RecordCommitment as CommitmentScheme>::Output],

        // Other stuff
        predicate_comm: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Output,
        predicate_rand: &<C::PredicateVerificationKeyCommitment as CommitmentScheme>::Randomness,

        local_data_comm: &<C::LocalDataCommitment as CommitmentScheme>::Output,
        local_data_rand: &<C::LocalDataCommitment as CommitmentScheme>::Randomness,

        memo: &[u8; 32],
        auxiliary: &[u8; 32],
        binding_signature: &BindingSignature,
    ) -> Self {
        let num_input_records = C::NUM_INPUT_RECORDS;
        let num_output_records = C::NUM_OUTPUT_RECORDS;

        assert_eq!(num_input_records, old_records.len());
        assert_eq!(num_input_records, old_witnesses.len());
        assert_eq!(num_input_records, old_address_secret_keys.len());
        assert_eq!(num_input_records, old_serial_numbers.len());

        assert_eq!(num_output_records, new_records.len());
        assert_eq!(num_output_records, new_sn_nonce_randomness.len());
        assert_eq!(num_output_records, new_commitments.len());

        Self {
            // Parameters
            comm_crh_sig_parameters: Some(comm_crh_sig_parameters.clone()),
            ledger_parameters: Some(ledger_parameters.clone()),

            // Digest
            ledger_digest: Some(ledger_digest.clone()),

            // Input records
            old_records: Some(old_records.to_vec()),
            old_witnesses: Some(old_witnesses.to_vec()),
            old_address_secret_keys: Some(old_address_secret_keys.to_vec()),
            old_serial_numbers: Some(old_serial_numbers.to_vec()),

            // Output records
            new_records: Some(new_records.to_vec()),
            new_sn_nonce_randomness: Some(new_sn_nonce_randomness.to_vec()),
            new_commitments: Some(new_commitments.to_vec()),

            // Other stuff
            predicate_comm: Some(predicate_comm.clone()),
            predicate_rand: Some(predicate_rand.clone()),

            local_data_comm: Some(local_data_comm.clone()),
            local_data_rand: Some(local_data_rand.clone()),

            memo: Some(memo.clone()),
            auxiliary: Some(auxiliary.clone()),
            binding_signature: Some(binding_signature.clone()),
        }
    }
}

impl<C: DelegablePaymentDPCComponents> ConstraintSynthesizer<C::InnerField> for InnerCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_core_checks_gadget::<C, CS>(
            cs,
            // Params
            self.comm_crh_sig_parameters.get()?,
            self.ledger_parameters.get()?,
            // digest
            self.ledger_digest.get()?,
            // old records
            self.old_records.get()?,
            self.old_witnesses.get()?,
            self.old_address_secret_keys.get()?,
            self.old_serial_numbers.get()?,
            // new records
            self.new_records.get()?,
            self.new_sn_nonce_randomness.get()?,
            self.new_commitments.get()?,
            // other stuff
            self.predicate_comm.get()?,
            self.predicate_rand.get()?,
            self.local_data_comm.get()?,
            self.local_data_rand.get()?,
            self.memo.get()?,
            self.auxiliary.get()?,
            self.binding_signature.get()?,
        )?;
        Ok(())
    }
}
