use crate::{
    base_dpc::{parameters::SystemParameters, record::DPCRecord, record_payload::RecordPayload, *},
    Assignment,
};
use snarkos_algorithms::commitment_tree::CommitmentMerklePath;
use snarkos_errors::gadgets::SynthesisError;
use snarkos_gadgets::algorithms::commitment_tree::merkle_path::CommitmentMerklePathGadget;
use snarkos_models::{
    algorithms::{CommitmentScheme, SignatureScheme, CRH},
    dpc::Record,
    gadgets::{
        algorithms::{CRHGadget, CommitmentGadget, SignaturePublicKeyRandomizationGadget},
        r1cs::{ConstraintSynthesizer, ConstraintSystem},
        utilities::{
            alloc::AllocGadget,
            boolean::Boolean,
            eq::EqGadget,
            select::CondSelectGadget,
            uint::UInt8,
            ToBytesGadget,
        },
    },
};
use snarkos_utilities::{to_bytes, ToBytes};

pub struct DummyCircuit<C: BaseDPCComponents> {
    /// System parameters
    pub system_parameters: Option<SystemParameters<C>>,

    /// Local data commitment path
    pub local_data_merkle_path: Option<CommitmentMerklePath<C::LocalDataCommitment, C::LocalDataCRH>>,

    /// Commitment to the program input.
    pub local_data_root: Option<<C::LocalDataCRH as CRH>::Output>,

    /// Record associated with the given position
    pub record: Option<DPCRecord<C>>,

    /// The old serial number of records being spend (May or may not be relevant if the record is new)
    pub old_serial_numbers: Option<Vec<<C::AccountSignature as SignatureScheme>::PublicKey>>,

    /// Local data commitment randomizer to derive the root
    pub local_data_commitment_randomizer: Option<<C::LocalDataCommitment as CommitmentScheme>::Randomness>,

    /// Transaction memo
    pub memo: Option<[u8; 32]>,

    /// Transaction network id
    pub network_id: Option<u8>,

    /// Record position
    pub position: u8,
}

impl<C: BaseDPCComponents> DummyCircuit<C> {
    pub fn blank(system_parameters: &SystemParameters<C>) -> Self {
        let local_data_root = <C::LocalDataCRH as CRH>::Output::default();
        let record = DPCRecord::default();
        let local_data_commitment_randomizer = <C::LocalDataCommitment as CommitmentScheme>::Randomness::default();

        let leaves = (
            <C::LocalDataCommitment as CommitmentScheme>::Output::default(),
            <C::LocalDataCommitment as CommitmentScheme>::Output::default(),
        );
        let inner_hashes = (
            <C::LocalDataCRH as CRH>::Output::default(),
            <C::LocalDataCRH as CRH>::Output::default(),
        );
        let local_data_merkle_path = CommitmentMerklePath {
            leaves,
            inner_hashes,
            parameters: system_parameters.local_data_crh.clone(),
        };

        let old_serial_numbers =
            vec![<C::AccountSignature as SignatureScheme>::PublicKey::default(); C::NUM_INPUT_RECORDS];

        Self {
            system_parameters: Some(system_parameters.clone()),
            local_data_root: Some(local_data_root),
            record: Some(record),
            local_data_merkle_path: Some(local_data_merkle_path.clone()),
            old_serial_numbers: Some(old_serial_numbers),
            local_data_commitment_randomizer: Some(local_data_commitment_randomizer),
            memo: Some([0u8; 32]),
            network_id: Some(0),
            position: 0u8,
        }
    }

    pub fn new(local_data: &LocalData<C>, position: u8) -> Self {
        assert!((position as usize) < (C::NUM_INPUT_RECORDS + C::NUM_OUTPUT_RECORDS));
        let records = [&local_data.old_records[..], &local_data.new_records[..]].concat();
        let record = &records[position as usize];
        let local_data_commitment_randomizer = &local_data.local_data_commitment_randomizers[position as usize];

        let leaf = &local_data.local_data_merkle_tree.leaves()[position as usize];
        let local_data_merkle_path = local_data.local_data_merkle_tree.generate_proof(leaf).unwrap();

        Self {
            system_parameters: Some(local_data.system_parameters.clone()),
            local_data_merkle_path: Some(local_data_merkle_path),
            local_data_root: Some(local_data.local_data_merkle_tree.root()),
            record: Some(record.clone()),
            old_serial_numbers: Some(local_data.old_serial_numbers.clone()),
            local_data_commitment_randomizer: Some(local_data_commitment_randomizer.clone()),
            memo: Some(local_data.memorandum),
            network_id: Some(local_data.network_id),
            position,
        }
    }
}

impl<C: BaseDPCComponents> ConstraintSynthesizer<C::InnerField> for DummyCircuit<C> {
    fn generate_constraints<CS: ConstraintSystem<C::InnerField>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        execute_dummy_check_gadget(
            cs,
            self.system_parameters.get()?,
            self.local_data_merkle_path.get()?,
            self.local_data_root.get()?,
            self.record.get()?,
            self.old_serial_numbers.get()?,
            self.local_data_commitment_randomizer.get()?,
            self.memo.get()?,
            *self.network_id.get()?,
            self.position,
        )
    }
}

/// Enforce that if a record is a dummy, that the value is 0 and payload is 0.
pub fn execute_dummy_check_gadget<C: BaseDPCComponents, CS: ConstraintSystem<C::InnerField>>(
    cs: &mut CS,
    system_parameters: &SystemParameters<C>,
    local_data_merkle_path: &CommitmentMerklePath<C::LocalDataCommitment, C::LocalDataCRH>,
    local_data_root: &<C::LocalDataCRH as CRH>::Output,
    record: &DPCRecord<C>,
    old_serial_numbers: &Vec<<C::AccountSignature as SignatureScheme>::PublicKey>,
    local_data_commitment_randomizer: &<C::LocalDataCommitment as CommitmentScheme>::Randomness,
    memo: &[u8; 32],
    network_id: u8,
    position: u8,
) -> Result<(), SynthesisError> {
    // Allocate the position
    let _position_gadget = UInt8::alloc_input_vec(cs.ns(|| "Alloc position"), &[position])?;

    // Allocate the parameters and local data root
    let local_data_commitment_parameters_gadget =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::ParametersGadget::alloc_input(
            &mut cs.ns(|| "Declare local data commitment parameters"),
            || Ok(system_parameters.local_data_commitment.parameters().clone()),
        )?;

    let local_data_crh_parameters = <C::LocalDataCRHGadget as CRHGadget<_, _>>::ParametersGadget::alloc(
        &mut cs.ns(|| "Declare local data CRH parameters"),
        || Ok(system_parameters.local_data_crh.parameters()),
    )?;

    let local_data_root_gadget = <C::LocalDataCRHGadget as CRHGadget<_, _>>::OutputGadget::alloc_input(
        cs.ns(|| "Allocate local data root"),
        || Ok(local_data_root),
    )?;

    let memo = UInt8::alloc_vec(cs.ns(|| "Allocate memorandum"), memo)?;
    let network_id = UInt8::alloc_vec(cs.ns(|| "Allocate network id"), &[network_id])?;

    // Enforce that the value is 0 and the payload is empty

    let zero_value = UInt8::constant_vec(&to_bytes![0u64]?);
    let empty_payload = UInt8::constant_vec(&to_bytes![RecordPayload::default()]?);

    let given_payload = UInt8::alloc_vec(&mut cs.ns(|| "given_payload"), &record.payload().to_bytes())?;
    let given_value = UInt8::alloc_vec(&mut cs.ns(|| "given_value"), &to_bytes![record.value()]?)?;

    given_value.enforce_equal(&mut cs.ns(|| "Enforce that the record has a zero value"), &zero_value)?;

    given_payload.enforce_equal(
        &mut cs.ns(|| "Enforce that the record has an empty payload"),
        &empty_payload,
    )?;

    // Enforce that the local data commitment leaf is valid for the root

    // Create the record commitment gadget

    let is_death_record = position < (C::NUM_INPUT_RECORDS as u8);
    let is_death = Boolean::alloc(&mut cs.ns(|| "is_death_record"), || Ok(is_death_record))?;

    let serial_number_position = position % 2;
    let serial_number = &old_serial_numbers[serial_number_position as usize];
    let serial_number_gadget = <C::AccountSignatureGadget as SignaturePublicKeyRandomizationGadget<
        C::AccountSignature,
        _,
    >>::PublicKeyGadget::alloc(&mut cs.ns(|| "Declare given serial number"), || {
        Ok(serial_number)
    })?;
    let serial_number_bytes = serial_number_gadget.to_bytes(&mut cs.ns(|| "serial_number_bytes"))?;

    let record_commitment_gadget =
        <C::RecordCommitmentGadget as CommitmentGadget<C::RecordCommitment, _>>::OutputGadget::alloc(
            &mut cs.ns(|| "given_commitment"),
            || Ok(record.commitment().clone()),
        )?;
    let record_commitment_bytes = record_commitment_gadget.to_bytes(&mut cs.ns(|| "record_commitment_bytes"))?;

    let mut death_input_bytes = vec![];

    death_input_bytes.extend_from_slice(&serial_number_bytes);
    death_input_bytes.extend_from_slice(&record_commitment_bytes);
    death_input_bytes.extend_from_slice(&memo);
    death_input_bytes.extend_from_slice(&network_id);

    let mut birth_input_bytes = vec![];

    birth_input_bytes.extend_from_slice(&record_commitment_bytes);
    birth_input_bytes.extend_from_slice(&memo);
    birth_input_bytes.extend_from_slice(&network_id);

    let local_data_commitment_randomness =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::RandomnessGadget::alloc(
            cs.ns(|| "Allocate record local data commitment randomness"),
            || Ok(local_data_commitment_randomizer),
        )?;

    // Create the local data commitment leaf for a death record

    let death_local_data_commtiment_leaf =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::check_commitment_gadget(
            cs.ns(|| "Commit to record local data - death"),
            &local_data_commitment_parameters_gadget,
            &death_input_bytes,
            &local_data_commitment_randomness,
        )?;

    // Create the local data commitment leaf for a birth record

    let birth_local_data_commtiment_leaf =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::check_commitment_gadget(
            cs.ns(|| "Commit to record local data - birth"),
            &local_data_commitment_parameters_gadget,
            &birth_input_bytes,
            &local_data_commitment_randomness,
        )?;

    // Select the local data commitment leaf based on the given position

    let local_data_commtiment_leaf =
        <C::LocalDataCommitmentGadget as CommitmentGadget<_, _>>::OutputGadget::conditionally_select(
            cs.ns(|| "conditionally_select the local_data_commitment_leaf"),
            &is_death,
            &death_local_data_commtiment_leaf,
            &birth_local_data_commtiment_leaf,
        )?;

    // Alloc the witness gadget. - Currently we do not have witnesses because the root is generated from scratch

    let witness_gadget =
        CommitmentMerklePathGadget::<_, _, C::LocalDataCommitmentGadget, C::LocalDataCRHGadget, _>::alloc(
            &mut cs.ns(|| "Declare local data witness path"),
            || Ok(local_data_merkle_path),
        )?;

    // Enforce that record commitment and witness is correct given the root

    witness_gadget.check_membership(
        &mut cs.ns(|| "Perform local data commitment membership witness check"),
        &local_data_crh_parameters,
        &local_data_root_gadget,
        &local_data_commtiment_leaf,
    )?;

    Ok(())
}
