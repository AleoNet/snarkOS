use snarkos_dpc::base_dpc::{
    instantiated::{CommitmentMerkleParameters, Components, InstantiatedDPC, Tx},
    parameters::PublicParameters,
    program::NoopProgram,
    record::DPCRecord,
    record_payload::RecordPayload,
    BaseDPCComponents,
};
use snarkos_errors::dpc::DPCError;
use snarkos_models::{
    algorithms::CRH,
    dpc::{DPCComponents, DPCScheme, Program},
};
use snarkos_objects::account::*;
use snarkos_storage::Ledger;
use snarkos_utilities::{to_bytes, ToBytes};

pub type MerkleTreeLedger = Ledger<Tx, CommitmentMerkleParameters>;

use rand::Rng;

/// Create a dummy transaction and return encoded transaction and output records
pub fn create_dummy_transaction<R: Rng>(rng: &mut R) -> Result<(Tx, Vec<DPCRecord<Components>>), DPCError> {
    let network_id = 1;

    let mut path = std::env::current_dir()?;
    path.push("storage_db");

    let ledger = MerkleTreeLedger::open_at_path(&path).unwrap();

    let parameters = PublicParameters::<Components>::load(false).unwrap();

    let new_spender = AccountPrivateKey::<Components>::new(
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
    let mut old_account_private_keys = vec![];

    while old_records.len() < Components::NUM_OUTPUT_RECORDS {
        let sn_randomness: [u8; 32] = rng.gen();
        let old_sn_nonce = parameters.system_parameters.serial_number_nonce.hash(&sn_randomness)?;

        let private_key = new_spender.clone();
        let address = AccountAddress::<Components>::from_private_key(
            parameters.account_signature_parameters(),
            parameters.account_commitment_parameters(),
            parameters.account_encryption_parameters(),
            &private_key,
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
        old_account_private_keys.push(private_key);
    }

    assert_eq!(old_records.len(), Components::NUM_INPUT_RECORDS);
    assert_eq!(old_account_private_keys.len(), Components::NUM_INPUT_RECORDS);

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

    // Construct the program proofs

    let local_data = execute_context.into_local_data();

    let noop_program = NoopProgram::<_, <Components as BaseDPCComponents>::NoopProgramSNARK>::new(noop_program_id);

    let mut old_death_program_proofs = vec![];
    for i in 0..Components::NUM_INPUT_RECORDS {
        let private_input = noop_program.execute(
            &parameters.noop_program_snark_parameters.proving_key,
            &parameters.noop_program_snark_parameters.verification_key,
            &local_data,
            i as u8,
            rng,
        )?;

        old_death_program_proofs.push(private_input);
    }

    let mut new_birth_program_proofs = vec![];
    for j in 0..Components::NUM_OUTPUT_RECORDS {
        let private_input = noop_program.execute(
            &parameters.noop_program_snark_parameters.proving_key,
            &parameters.noop_program_snark_parameters.verification_key,
            &local_data,
            (Components::NUM_INPUT_RECORDS + j) as u8,
            rng,
        )?;

        new_birth_program_proofs.push(private_input);
    }

    // Online execution to generate a DPC transaction
    let (new_records, transaction) = InstantiatedDPC::execute_online(
        &parameters,
        execute_context,
        &old_death_program_proofs,
        &new_birth_program_proofs,
        &ledger,
        rng,
    )?;

    //

    drop(ledger);
    MerkleTreeLedger::destroy_storage(path).unwrap();

    Ok((transaction, new_records))
}
