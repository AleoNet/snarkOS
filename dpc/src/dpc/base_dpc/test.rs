use super::instantiated::*;
use crate::dpc::base_dpc::{
    binding_signature::*,
    execute_inner_proof_gadget,
    execute_outer_proof_gadget,
    inner_circuit::InnerCircuit,
    program::{PrivateProgramInput, ProgramCircuit},
    record_payload::RecordPayload,
    records::record_encryption::*,
    BaseDPCComponents,
    ExecuteContext,
    DPC,
};
use snarkos_algorithms::merkle_tree::MerklePath;
use snarkos_curves::bls12_377::{Fq, Fr};
use snarkos_models::{
    algorithms::{CommitmentScheme, MerkleParameters, CRH, SNARK},
    dpc::Record,
    gadgets::r1cs::{ConstraintSystem, TestConstraintSystem},
    objects::{AccountScheme, LedgerScheme},
};

use snarkos_objects::{
    dpc::DPCTransactions,
    Account,
    Block,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
    PedersenMerkleRootHash,
    ProofOfSuccinctWork,
};
use snarkos_testing::storage::*;
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};

use itertools::Itertools;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

#[cfg(debug_assertions)]
use snarkos_algorithms::snark::gm17::PreparedVerifyingKey;

#[cfg(debug_assertions)]
use crate::dpc::base_dpc::program::ProgramLocalData;

type L = Ledger<Tx, CommitmentMerkleParameters>;

#[test]
fn test_execute_base_dpc_constraints() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    // Specify network_id
    let network_id: u8 = 0;

    // Generate parameters for the ledger, commitment schemes, CRH, and the
    // "always-accept" program.
    let ledger_parameters = CommitmentMerkleParameters::setup(&mut rng);
    let system_parameters = InstantiatedDPC::generate_system_parameters(&mut rng).unwrap();
    let program_snark_pp = InstantiatedDPC::generate_program_snark_parameters(&system_parameters, &mut rng).unwrap();
    #[cfg(debug_assertions)]
    let program_snark_pvk: PreparedVerifyingKey<_> = program_snark_pp.verification_key.clone().into();

    let program_snark_vk_bytes = to_bytes![
        ProgramVerificationKeyHash::hash(
            &system_parameters.program_verification_key_hash,
            &to_bytes![program_snark_pp.verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let signature_parameters = &system_parameters.account_signature;
    let commitment_parameters = &system_parameters.account_commitment;
    let encryption_parameters = &system_parameters.account_encryption;

    // Generate metadata and an account for a dummy initial record.
    let dummy_account = Account::new(
        signature_parameters,
        commitment_parameters,
        encryption_parameters,
        &mut rng,
    )
    .unwrap();

    let genesis_block = Block {
        header: BlockHeader {
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            merkle_root_hash: MerkleRootHash([0u8; 32]),
            time: 0,
            difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
            nonce: 0,
            pedersen_merkle_root_hash: PedersenMerkleRootHash([0u8; 32]),
            proof: ProofOfSuccinctWork::default(),
        },
        transactions: DPCTransactions::new(),
    };

    // Use genesis record, serial number, and memo to initialize the ledger.
    let ledger = initialize_test_blockchain::<Tx, CommitmentMerkleParameters>(ledger_parameters, genesis_block);

    let sn_nonce = SerialNumberNonce::hash(&system_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
    let old_record = DPC::generate_record(
        &system_parameters,
        &sn_nonce,
        &dummy_account.address,
        true,
        0,
        &RecordPayload::default(),
        &Program::new(program_snark_vk_bytes.clone()),
        &Program::new(program_snark_vk_bytes.clone()),
        &mut rng,
    )
    .unwrap();

    // Set the input records for our transaction to be the initial dummy records.
    let old_records = vec![old_record.clone(); NUM_INPUT_RECORDS];
    let old_account_private_keys = vec![dummy_account.private_key.clone(); NUM_INPUT_RECORDS];

    // Construct new records.

    // Create an account for an actual new record.

    let new_account = Account::new(
        signature_parameters,
        commitment_parameters,
        encryption_parameters,
        &mut rng,
    )
    .unwrap();

    // Set the new record's program to be the "always-accept" program.
    let new_program = Program::new(program_snark_vk_bytes.clone());

    let new_record_owners = vec![new_account.address.clone(); NUM_OUTPUT_RECORDS];
    let new_is_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];
    let new_values = vec![10; NUM_OUTPUT_RECORDS];
    let new_payloads = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];
    let new_birth_programs = vec![new_program.clone(); NUM_OUTPUT_RECORDS];
    let new_death_programs = vec![new_program.clone(); NUM_OUTPUT_RECORDS];
    let memo = [0u8; 32];

    let context = InstantiatedDPC::execute_helper::<L, _>(
        &system_parameters,
        &old_records,
        &old_account_private_keys,
        &new_record_owners,
        &new_is_dummy_flags,
        &new_values,
        &new_payloads,
        &new_birth_programs,
        &new_death_programs,
        &memo,
        network_id,
        &mut rng,
    )
    .unwrap();

    let ExecuteContext {
        system_parameters: _system_parameters,

        old_records,
        old_account_private_keys,
        old_serial_numbers,
        old_randomizers: _,

        new_records,
        new_sn_nonce_randomness,
        new_commitments,

        new_records_encryption_randomness,
        new_encrypted_records: _new_encrypted_records,
        new_encrypted_record_hashes,

        program_commitment,
        program_randomness,
        local_data_root: local_data_commitment,
        local_data_commitment_randomizers,
        value_balance,
        memorandum,
        network_id,
    } = context;

    // Generate the program proofs

    let mut old_proof_and_vk = vec![];
    for i in 0..NUM_INPUT_RECORDS {
        let proof = ProgramSNARK::prove(
            &program_snark_pp.proving_key,
            ProgramCircuit::new(&system_parameters, &local_data_commitment, i as u8),
            &mut rng,
        )
        .expect("Proof should work");
        #[cfg(debug_assertions)]
        {
            let program_pub_input: ProgramLocalData<Components> = ProgramLocalData {
                local_data_commitment_parameters: system_parameters.local_data_commitment.parameters().clone(),
                local_data_root: local_data_commitment.clone(),
                position: i as u8,
            };
            assert!(ProgramSNARK::verify(&program_snark_pvk, &program_pub_input, &proof).expect("Proof should verify"));
        }
        let private_input: PrivateProgramInput<_> = PrivateProgramInput {
            verification_key: program_snark_pp.verification_key.clone(),
            proof,
        };
        old_proof_and_vk.push(private_input);
    }

    // Construct the ledger witnesses

    let ledger_digest = ledger.digest().expect("could not get digest");

    // Generate the ledger membership witnesses
    let mut old_witnesses = Vec::with_capacity(NUM_INPUT_RECORDS);

    // Compute the ledger membership witness and serial number from the old records.
    for record in old_records.iter() {
        if record.is_dummy() {
            old_witnesses.push(MerklePath::default());
        } else {
            let witness = ledger.prove_cm(&record.commitment()).unwrap();
            old_witnesses.push(witness);
        }
    }

    let mut new_proof_and_vk = vec![];
    for j in 0..NUM_OUTPUT_RECORDS {
        let proof = ProgramSNARK::prove(
            &program_snark_pp.proving_key,
            ProgramCircuit::new(&system_parameters, &local_data_commitment, j as u8),
            &mut rng,
        )
        .expect("Proof should work");

        #[cfg(debug_assertions)]
        {
            let program_pub_input: ProgramLocalData<Components> = ProgramLocalData {
                local_data_commitment_parameters: system_parameters.local_data_commitment.parameters().clone(),
                local_data_root: local_data_commitment.clone(),
                position: j as u8,
            };
            assert!(ProgramSNARK::verify(&program_snark_pvk, &program_pub_input, &proof).expect("Proof should verify"));
        }

        let private_input: PrivateProgramInput<_> = PrivateProgramInput {
            verification_key: program_snark_pp.verification_key.clone(),
            proof,
        };
        new_proof_and_vk.push(private_input);
    }

    // Generate binding signature

    // Generate value commitments for input records

    let mut old_value_commits = vec![];
    let mut old_value_commit_randomness = vec![];

    for old_record in &old_records {
        // If the record is a dummy, then the value should be 0
        let input_value = match old_record.is_dummy() {
            true => 0,
            false => old_record.value(),
        };

        // Generate value commitment randomness
        let value_commitment_randomness =
            <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

        // Generate the value commitment
        let value_commitment = system_parameters
            .value_commitment
            .commit(&input_value.to_le_bytes(), &value_commitment_randomness)
            .unwrap();

        old_value_commits.push(value_commitment);
        old_value_commit_randomness.push(value_commitment_randomness);
    }

    // Generate value commitments for output records

    let mut new_value_commits = vec![];
    let mut new_value_commit_randomness = vec![];

    for new_record in &new_records {
        // If the record is a dummy, then the value should be 0
        let output_value = match new_record.is_dummy() {
            true => 0,
            false => new_record.value(),
        };

        // Generate value commitment randomness
        let value_commitment_randomness =
            <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

        // Generate the value commitment
        let value_commitment = system_parameters
            .value_commitment
            .commit(&output_value.to_le_bytes(), &value_commitment_randomness)
            .unwrap();

        new_value_commits.push(value_commitment);
        new_value_commit_randomness.push(value_commitment_randomness);
    }

    let sighash = to_bytes![local_data_commitment].unwrap();

    let binding_signature = create_binding_signature::<
        <Components as BaseDPCComponents>::ValueCommitment,
        <Components as BaseDPCComponents>::BindingSignatureGroup,
        _,
    >(
        &system_parameters.value_commitment,
        &old_value_commits,
        &new_value_commits,
        &old_value_commit_randomness,
        &new_value_commit_randomness,
        value_balance,
        &sighash,
        &mut rng,
    )
    .unwrap();

    // Prepare record encryption components used in the inner SNARK

    let mut new_records_encryption_gadget_components = Vec::with_capacity(NUM_OUTPUT_RECORDS);
    for (record, ciphertext_randomness) in new_records.iter().zip_eq(&new_records_encryption_randomness) {
        let record_encryption_gadget_components =
            RecordEncryption::prepare_encryption_gadget_components(&system_parameters, &record, ciphertext_randomness)
                .unwrap();

        new_records_encryption_gadget_components.push(record_encryption_gadget_components);
    }

    //////////////////////////////////////////////////////////////////////////
    // Check that the core check constraint system was satisfied.
    let mut core_cs = TestConstraintSystem::<Fr>::new();

    execute_inner_proof_gadget::<_, _>(
        &mut core_cs.ns(|| "Core checks"),
        &system_parameters,
        ledger.parameters(),
        &ledger_digest,
        &old_records,
        &old_witnesses,
        &old_account_private_keys,
        &old_serial_numbers,
        &new_records,
        &new_sn_nonce_randomness,
        &new_commitments,
        &new_records_encryption_randomness,
        &new_records_encryption_gadget_components,
        &new_encrypted_record_hashes,
        &program_commitment,
        &program_randomness,
        &local_data_commitment,
        &local_data_commitment_randomizers,
        &memorandum,
        &old_value_commits,
        &old_value_commit_randomness,
        &new_value_commits,
        &new_value_commit_randomness,
        value_balance,
        &binding_signature,
        network_id,
    )
    .unwrap();

    if !core_cs.is_satisfied() {
        println!("=========================================================");
        println!("num constraints: {:?}", core_cs.num_constraints());
        println!("Unsatisfied constraints:");
        println!("{}", core_cs.which_is_unsatisfied().unwrap());
        println!("=========================================================");
    }

    if core_cs.is_satisfied() {
        println!("\n\n\n\nAll Core check constraints:");
        //        core_cs.print_named_objects();
        println!("num constraints: {:?}", core_cs.num_constraints());
    }
    println!("=========================================================");
    println!("=========================================================");
    println!("=========================================================\n\n\n");

    assert!(core_cs.is_satisfied());

    // Generate inner snark parameters and proof for verification in the outer snark
    let inner_snark_parameters = <Components as BaseDPCComponents>::InnerSNARK::setup(
        InnerCircuit::blank(&system_parameters, ledger.parameters()),
        &mut rng,
    )
    .unwrap();

    let inner_snark_proof = <Components as BaseDPCComponents>::InnerSNARK::prove(
        &inner_snark_parameters.0,
        InnerCircuit::new(
            &system_parameters,
            ledger.parameters(),
            &ledger_digest,
            &old_records,
            &old_witnesses,
            &old_account_private_keys,
            &old_serial_numbers,
            &new_records,
            &new_sn_nonce_randomness,
            &new_commitments,
            &new_records_encryption_randomness,
            &new_records_encryption_gadget_components,
            &new_encrypted_record_hashes,
            &program_commitment,
            &program_randomness,
            &local_data_commitment,
            &local_data_commitment_randomizers,
            &memorandum,
            &old_value_commits,
            &old_value_commit_randomness,
            &new_value_commits,
            &new_value_commit_randomness,
            value_balance,
            &binding_signature,
            network_id,
        ),
        &mut rng,
    )
    .unwrap();

    let inner_snark_vk: <<Components as BaseDPCComponents>::InnerSNARK as SNARK>::VerificationParameters =
        inner_snark_parameters.1.clone().into();

    // Check that the proof check constraint system was satisfied.
    let mut pf_check_cs = TestConstraintSystem::<Fq>::new();

    execute_outer_proof_gadget::<_, _>(
        &mut pf_check_cs.ns(|| "Check program proofs"),
        &system_parameters,
        ledger.parameters(),
        &ledger_digest,
        &old_serial_numbers,
        &new_commitments,
        &new_encrypted_record_hashes,
        &memorandum,
        value_balance,
        network_id,
        &inner_snark_vk,
        &inner_snark_proof,
        &old_proof_and_vk,
        &new_proof_and_vk,
        &program_commitment,
        &program_randomness,
        &local_data_commitment,
    )
    .unwrap();

    if !pf_check_cs.is_satisfied() {
        println!("=========================================================");
        println!("num constraints: {:?}", pf_check_cs.num_constraints());
        println!("Unsatisfied constraints:");
        println!("{}", pf_check_cs.which_is_unsatisfied().unwrap());
        println!("=========================================================");
    }
    if pf_check_cs.is_satisfied() {
        println!("\n\n\n\nAll Proof check constraints:");
        // pf_check_cs.print_named_objects();
        println!("num constraints: {:?}", pf_check_cs.num_constraints());
    }
    println!("=========================================================");
    println!("=========================================================");
    println!("=========================================================");

    assert!(pf_check_cs.is_satisfied());

    let verify_binding_signature = verify_binding_signature::<
        <Components as BaseDPCComponents>::ValueCommitment,
        <Components as BaseDPCComponents>::BindingSignatureGroup,
    >(
        &system_parameters.value_commitment,
        &old_value_commits,
        &new_value_commits,
        value_balance,
        &sighash,
        &binding_signature,
    )
    .unwrap();

    assert!(verify_binding_signature);

    kill_storage(ledger);
}
