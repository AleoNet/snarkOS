use crate::transaction::delegate_transaction;
use snarkos_dpc::base_dpc::{
    instantiated::{CommitmentMerkleParameters, Components, InstantiatedDPC, Tx},
    parameters::PublicParameters,
    record::DPCRecord,
    record_payload::RecordPayload,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::CRH,
    dpc::{DPCComponents, DPCScheme},
};
use snarkos_objects::account::*;
use snarkos_storage::Ledger;
use snarkos_utilities::{to_bytes, ToBytes};

pub type MerkleTreeLedger = Ledger<Tx, CommitmentMerkleParameters>;

use rand::Rng;

/// Create a dummy transaction and return encoded transaction and output records
pub fn create_dummy_transaction<R: Rng>(
    network_id: u8,
    rng: &mut R,
) -> Result<(Tx, Vec<DPCRecord<Components>>), DPCError> {
    let parameters = PublicParameters::<Components>::load(false).unwrap();

    let spender = AccountPrivateKey::<Components>::new(
        &parameters.system_parameters.account_signature,
        &parameters.system_parameters.account_commitment,
        rng,
    )?;

    let new_recipient_private_key = AccountPrivateKey::<Components>::new(
        &parameters.system_parameters.account_signature,
        &parameters.system_parameters.account_commitment,
        rng,
    )?;
    let new_recipient = AccountAddress::<Components>::from_private_key(
        parameters.account_signature_parameters(),
        parameters.account_commitment_parameters(),
        parameters.account_encryption_parameters(),
        &new_recipient_private_key,
    )?;

    let noop_program_id = to_bytes![
        parameters
            .system_parameters
            .program_verification_key_hash
            .hash(&to_bytes![parameters.noop_program_snark_parameters.verification_key]?)?
    ]?;

    let mut old_records = vec![];
    let old_account_private_keys = vec![spender.clone(); Components::NUM_INPUT_RECORDS];

    while old_records.len() < Components::NUM_INPUT_RECORDS {
        let sn_randomness: [u8; 32] = rng.gen();
        let old_sn_nonce = parameters.system_parameters.serial_number_nonce.hash(&sn_randomness)?;

        let address = AccountAddress::<Components>::from_private_key(
            parameters.account_signature_parameters(),
            parameters.account_commitment_parameters(),
            parameters.account_encryption_parameters(),
            &spender,
        )?;

        let dummy_record = InstantiatedDPC::generate_record(
            &parameters.system_parameters,
            &old_sn_nonce,
            &address,
            true, // The input record is dummy
            0,
            &RecordPayload::default(),
            &noop_program_id,
            &noop_program_id,
            rng,
        )?;

        old_records.push(dummy_record);
    }

    assert_eq!(old_records.len(), Components::NUM_INPUT_RECORDS);

    let new_record_owners = vec![new_recipient.clone(); Components::NUM_OUTPUT_RECORDS];
    let new_is_dummy_flags = vec![true; Components::NUM_OUTPUT_RECORDS];
    let new_values = vec![0; Components::NUM_OUTPUT_RECORDS];
    let new_birth_program_ids = vec![noop_program_id.clone(); Components::NUM_OUTPUT_RECORDS];
    let new_death_program_ids = vec![noop_program_id.clone(); Components::NUM_OUTPUT_RECORDS];
    let new_payloads = vec![RecordPayload::default(); Components::NUM_OUTPUT_RECORDS];

    // Generate a random memo
    let memo = rng.gen();

    // Generate transaction

    // Offline execution to generate a DPC transaction
    let execute_context = <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::execute_offline(
        &parameters.system_parameters,
        &old_records,
        &old_account_private_keys,
        &new_record_owners,
        &new_is_dummy_flags,
        &new_values,
        &new_payloads,
        &new_birth_program_ids,
        &new_death_program_ids,
        &memo,
        network_id,
        rng,
    )?;

    // Delegate online phase of transaction generation

    let mut path = std::env::current_dir()?;
    path.push("storage_db");

    let ledger = MerkleTreeLedger::open_at_path(&path).unwrap();

    let (transaction, new_records) = delegate_transaction(execute_context, &ledger, rng)?;

    drop(ledger);
    MerkleTreeLedger::destroy_storage(path).unwrap();

    Ok((transaction, new_records))
}
