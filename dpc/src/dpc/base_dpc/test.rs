use super::instantiated::*;
use crate::dpc::base_dpc::{
    binding_signature::*,
    execute_inner_proof_gadget,
    execute_outer_proof_gadget,
    inner_circuit::InnerCircuit,
    predicate::PrivatePredicateInput,
    predicate_circuit::{PredicateCircuit, PredicateLocalData},
    record_payload::RecordPayload,
    BaseDPCComponents,
    ExecuteContext,
    DPC,
};
use snarkos_algorithms::snark::PreparedVerifyingKey;
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

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

#[test]
fn test_execute_base_dpc_constraints() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    // Specify network_id
    let network_id: u8 = 0;

    // Generate parameters for the ledger, commitment schemes, CRH, and the
    // "always-accept" predicate.
    let ledger_parameters = CommitmentMerkleParameters::setup(&mut rng);
    let circuit_parameters = InstantiatedDPC::generate_circuit_parameters(&mut rng).unwrap();
    let pred_nizk_pp = InstantiatedDPC::generate_predicate_snark_parameters(&circuit_parameters, &mut rng).unwrap();
    #[cfg(debug_assertions)]
    let pred_nizk_pvk: PreparedVerifyingKey<_> = pred_nizk_pp.verification_key.clone().into();

    let pred_nizk_vk_bytes = to_bytes![
        PredicateVerificationKeyHash::hash(
            &circuit_parameters.predicate_verification_key_hash,
            &to_bytes![pred_nizk_pp.verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let signature_parameters = &circuit_parameters.account_signature;
    let commitment_parameters = &circuit_parameters.account_commitment;

    // Generate metadata and an account for a dummy initial record.
    let meta_data = [1u8; 32];
    let dummy_account = Account::new(signature_parameters, commitment_parameters, &meta_data, &mut rng).unwrap();

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

    let sn_nonce = SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
    let old_record = DPC::generate_record(
        &circuit_parameters,
        &sn_nonce,
        &dummy_account.public_key,
        true,
        0,
        &RecordPayload::default(),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &mut rng,
    )
    .unwrap();

    // Set the input records for our transaction to be the initial dummy records.
    let old_records = vec![old_record.clone(); NUM_INPUT_RECORDS];
    let old_account_private_keys = vec![dummy_account.private_key.clone(); NUM_INPUT_RECORDS];

    // Construct new records.

    // Create an account for an actual new record.

    let new_metadata = [1u8; 32];
    let new_account = Account::new(signature_parameters, commitment_parameters, &new_metadata, &mut rng).unwrap();

    // Set the new record's predicate to be the "always-accept" predicate.
    let new_predicate = Predicate::new(pred_nizk_vk_bytes.clone());

    let new_account_public_keys = vec![new_account.public_key.clone(); NUM_OUTPUT_RECORDS];
    let new_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];
    let new_values = vec![10; NUM_OUTPUT_RECORDS];
    let new_payloads = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];
    let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let memo = [0u8; 32];

    let context = DPC::execute_helper(
        &circuit_parameters,
        &old_records,
        &old_account_private_keys,
        &new_account_public_keys,
        &new_dummy_flags,
        &new_values,
        &new_payloads,
        &new_birth_predicates,
        &new_death_predicates,
        &memo,
        network_id,
        &ledger,
        &mut rng,
    )
    .unwrap();

    let ExecuteContext {
        circuit_parameters: _comm_crh_sig_pp,
        ledger_digest,

        old_records,
        old_witnesses,
        old_account_private_keys,
        old_serial_numbers,
        old_randomizers: _,

        new_records,
        new_sn_nonce_randomness,
        new_commitments,
        predicate_commitment: predicate_comm,
        predicate_randomness: predicate_rand,
        local_data_commitment_leaves,
        local_data_randomness,
        local_data_witnesses,
        local_data_commitment_digest,
        value_balance,
    } = context;

    // Generate the predicate proofs

    let mut old_proof_and_vk = vec![];
    for i in 0..NUM_INPUT_RECORDS {
        let proof = PredicateSNARK::prove(
            &pred_nizk_pp.proving_key,
            PredicateCircuit::new(&circuit_parameters, &local_data_commitment_digest, i as u8),
            &mut rng,
        )
        .expect("Proof should work");
        #[cfg(debug_assertions)]
        {
            let pred_pub_input: PredicateLocalData<Components> = PredicateLocalData {
                local_data_commitment_parameters: circuit_parameters.local_data_commitment.parameters().clone(),
                local_data_commitment: local_data_commitment_digest.clone(),
                position: i as u8,
            };
            assert!(PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify"));
        }
        let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
            verification_key: pred_nizk_pp.verification_key.clone(),
            proof,
        };
        old_proof_and_vk.push(private_input);
    }

    let mut new_proof_and_vk = vec![];
    for j in 0..NUM_OUTPUT_RECORDS {
        let proof = PredicateSNARK::prove(
            &pred_nizk_pp.proving_key,
            PredicateCircuit::new(&circuit_parameters, &local_data_commitment_digest, j as u8),
            &mut rng,
        )
        .expect("Proof should work");

        #[cfg(debug_assertions)]
        {
            let pred_pub_input: PredicateLocalData<Components> = PredicateLocalData {
                local_data_commitment_parameters: circuit_parameters.local_data_commitment.parameters().clone(),
                local_data_commitment: local_data_commitment_digest.clone(),
                position: j as u8,
            };
            assert!(PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify"));
        }

        let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
            verification_key: pred_nizk_pp.verification_key.clone(),
            proof,
        };
        new_proof_and_vk.push(private_input);
    }

    // Generate binding signature

    // Generate value commitments for input records

    let mut old_value_commits = vec![];
    let mut old_value_commit_randomness = vec![];

    for old_record in old_records {
        // If the record is a dummy, then the value should be 0
        let input_value = match old_record.is_dummy() {
            true => 0,
            false => old_record.value(),
        };

        // Generate value commitment randomness
        let value_commitment_randomness =
            <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

        // Generate the value commitment
        let value_commitment = circuit_parameters
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
        let value_commitment = circuit_parameters
            .value_commitment
            .commit(&output_value.to_le_bytes(), &value_commitment_randomness)
            .unwrap();

        new_value_commits.push(value_commitment);
        new_value_commit_randomness.push(value_commitment_randomness);
    }

    let sighash = to_bytes![local_data_commitment_digest].unwrap();

    let binding_signature = create_binding_signature::<
        <Components as BaseDPCComponents>::ValueCommitment,
        <Components as BaseDPCComponents>::BindingSignatureGroup,
        _,
    >(
        &circuit_parameters.value_commitment,
        &old_value_commits,
        &new_value_commits,
        &old_value_commit_randomness,
        &new_value_commit_randomness,
        value_balance,
        &sighash,
        &mut rng,
    )
    .unwrap();

    //////////////////////////////////////////////////////////////////////////
    // Check that the core check constraint system was satisfied.
    let mut core_cs = TestConstraintSystem::<Fr>::new();

    execute_inner_proof_gadget::<_, _>(
        &mut core_cs.ns(|| "Core checks"),
        &circuit_parameters,
        ledger.parameters(),
        &ledger_digest,
        &old_records,
        &old_witnesses,
        &old_account_private_keys,
        &old_serial_numbers,
        &new_records,
        &new_sn_nonce_randomness,
        &new_commitments,
        &predicate_comm,
        &predicate_rand,
        &local_data_commitment_leaves,
        &local_data_randomness,
        &local_data_witnesses,
        &local_data_commitment_digest,
        &memo,
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
        println!("Unsatisfied constraints:");
        println!("{}", core_cs.which_is_unsatisfied().unwrap());
        println!("=========================================================");
    }

    if core_cs.is_satisfied() {
        println!("\n\n\n\nAll Core check constraints:");
        core_cs.print_named_objects();
    }
    println!("=========================================================");
    println!("=========================================================");
    println!("=========================================================\n\n\n");

    assert!(core_cs.is_satisfied());

    // Generate inner snark parameters and proof for verification in the outer snark
    let inner_snark_parameters = <Components as BaseDPCComponents>::InnerSNARK::setup(
        InnerCircuit::blank(&circuit_parameters, ledger.parameters()),
        &mut rng,
    )
    .unwrap();

    let inner_snark_proof = <Components as BaseDPCComponents>::InnerSNARK::prove(
        &inner_snark_parameters.0,
        InnerCircuit::new(
            &circuit_parameters,
            ledger.parameters(),
            &ledger_digest,
            old_records,
            &old_witnesses,
            old_account_private_keys,
            &old_serial_numbers,
            &new_records,
            &new_sn_nonce_randomness,
            &new_commitments,
            &predicate_comm,
            &predicate_rand,
            &local_data_commitment_leaves,
            &local_data_randomness,
            &local_data_witnesses,
            &local_data_commitment_digest,
            &memo,
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
        &mut pf_check_cs.ns(|| "Check predicate proofs"),
        &circuit_parameters,
        ledger.parameters(),
        &ledger_digest,
        &old_serial_numbers,
        &new_commitments,
        &memo,
        value_balance,
        network_id,
        &inner_snark_vk,
        &inner_snark_proof,
        &old_proof_and_vk,
        &new_proof_and_vk,
        &predicate_comm,
        &predicate_rand,
        &local_data_commitment_digest,
    )
    .unwrap();

    if !pf_check_cs.is_satisfied() {
        println!("=========================================================");
        println!("Unsatisfied constraints:");
        println!("{}", pf_check_cs.which_is_unsatisfied().unwrap());
        println!("=========================================================");
    }
    if pf_check_cs.is_satisfied() {
        println!("\n\n\n\nAll Proof check constraints:");
        pf_check_cs.print_named_objects();
    }
    println!("=========================================================");
    println!("=========================================================");
    println!("=========================================================");

    assert!(pf_check_cs.is_satisfied());

    let verify_binding_signature = verify_binding_signature::<
        <Components as BaseDPCComponents>::ValueCommitment,
        <Components as BaseDPCComponents>::BindingSignatureGroup,
    >(
        &circuit_parameters.value_commitment,
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
