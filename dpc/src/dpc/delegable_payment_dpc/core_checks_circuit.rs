use crate::{
    constraints::{delegable_payment_dpc::execute_core_checks_gadget, Assignment},
    dpc::delegable_payment_dpc::{
        address::AddressSecretKey,
        binding_signature::BindingSignature,
        parameters::CommCRHSigPublicParameters,
        record::DPCRecord,
        DelegablePaymentDPCComponents,
    },
    ledger::MerkleTreeParams,
};
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTreeDigest};
use snarkos_errors::{curves::ConstraintFieldError, gadgets::SynthesisError};
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH},
    curves::to_field_vec::ToConstraintField,
    gadgets::r1cs::{ConstraintSynthesizer, ConstraintSystem},
};

pub struct CoreChecksVerifierInput<C: DelegablePaymentDPCComponents> {
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

impl<C: DelegablePaymentDPCComponents> ToConstraintField<C::InnerField> for CoreChecksVerifierInput<C>
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

#[derive(Derivative)]
#[derivative(Clone(bound = "C: DelegablePaymentDPCComponents"))]
pub struct CoreChecksCircuit<C: DelegablePaymentDPCComponents> {
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

impl<C: DelegablePaymentDPCComponents> CoreChecksCircuit<C> {
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

impl<C: DelegablePaymentDPCComponents> ConstraintSynthesizer<C::InnerField> for CoreChecksCircuit<C> {
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
