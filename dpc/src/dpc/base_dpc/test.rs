use super::instantiated::*;
use crate::dpc::base_dpc::{
    binding_signature::*,
    execute_inner_proof_gadget,
    execute_outer_proof_gadget,
    payment_circuit::{PaymentCircuit, PaymentPredicateLocalData},
    predicate::PrivatePredicateInput,
    record_payload::PaymentRecordPayload,
    BaseDPCComponents,
    ExecuteContext,
    DPC,
};
use snarkos_algorithms::snark::PreparedVerifyingKey;
use snarkos_curves::bls12_377::{Fq, Fr};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, SNARK},
    dpc::Record,
    gadgets::r1cs::{ConstraintSystem, TestConstraintSystem},
    objects::AccountScheme,
};
use snarkos_objects::{Account, Ledger};
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, to_bytes};

use rand::{Rng, SeedableRng};
use rand_xorshift::XorShiftRng;

#[test]
fn test_execute_base_dpc_constraints() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);
    // Generate parameters for the ledger, commitment schemes, CRH, and the
    // "always-accept" predicate.
    let ledger_parameters = MerkleTreeLedger::setup(&mut rng).expect("Ledger setup failed");
    let circuit_parameters = InstantiatedDPC::generate_circuit_parameters(&mut rng).unwrap();
    let pred_nizk_pp = InstantiatedDPC::generate_pred_nizk_parameters(&circuit_parameters, &mut rng).unwrap();
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

    let signature_parameters = &circuit_parameters.signature;
    let commitment_parameters = &circuit_parameters.account_commitment;

    // Generate metadata and an account for a dummy initial, or "genesis", record.
    let genesis_metadata = [1u8; 32];
    let genesis_account = Account::new(
        signature_parameters,
        commitment_parameters,
        &genesis_metadata,
        None,
        &mut rng,
    )
    .unwrap();

    let genesis_sn_nonce =
        SerialNumberNonce::hash(&circuit_parameters.serial_number_nonce, &[0u8; 1]).unwrap();
    let genesis_record = DPC::generate_record(
        &circuit_parameters,
        &genesis_sn_nonce,
        &genesis_account.public_key,
        true,
        &PaymentRecordPayload::default(),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &Predicate::new(pred_nizk_vk_bytes.clone()),
        &mut rng,
    )
    .unwrap();

    // Generate serial number for the genesis record.
    let (genesis_sn, _) = DPC::generate_sn(&circuit_parameters, &genesis_record, &genesis_account.private_key).unwrap();
    let genesis_memo = [0u8; 32];

    let mut path = std::env::temp_dir();
    let random_storage_path: usize = rng.gen();
    path.push(format!("test_execute_base_dpc_constraints{}", random_storage_path));

    // Use genesis record, serial number, and memo to initialize the ledger.
    let ledger = MerkleTreeLedger::new(
        &path,
        ledger_parameters,
        genesis_record.commitment(),
        genesis_sn.clone(),
        genesis_memo,
        pred_nizk_vk_bytes.to_vec(),
        to_bytes![genesis_account].unwrap().to_vec(),
    )
    .unwrap();

    // Set the input records for our transaction to be the initial dummy records.
    let old_records = vec![genesis_record.clone(); NUM_INPUT_RECORDS];
    let old_account_private_keys = vec![genesis_account.private_key.clone(); NUM_INPUT_RECORDS];

    // Construct new records.

    // Create an account for an actual new record.

    let new_metadata = [1u8; 32];
    let new_account = Account::new(
        signature_parameters,
        commitment_parameters,
        &new_metadata,
        None,
        &mut rng,
    )
    .unwrap();

    // Create a payload.
    let new_payload = PaymentRecordPayload { balance: 10, lock: 0 };

    // Set the new record's predicate to be the "always-accept" predicate.
    let new_predicate = Predicate::new(pred_nizk_vk_bytes.clone());

    let new_account_public_keys = vec![new_account.public_key.clone(); NUM_OUTPUT_RECORDS];
    let new_payloads = vec![new_payload.clone(); NUM_OUTPUT_RECORDS];
    let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let new_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];
    let auxiliary = [0u8; 32];
    let memo = [0u8; 32];

    let context = DPC::execute_helper(
        &circuit_parameters,
        &old_records,
        &old_account_private_keys,
        &new_account_public_keys,
        &new_dummy_flags,
        &new_payloads,
        &new_birth_predicates,
        &new_death_predicates,
        &memo,
        &auxiliary,
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
        local_data_commitment: local_data_comm,
        local_data_randomness: local_data_rand,
        value_balance,
    } = context;

    // Generate the predicate proofs

    let mut old_proof_and_vk = vec![];
    for i in 0..NUM_INPUT_RECORDS {
        // If the record is a dummy, then the value should be 0
        let value = match new_records[i].is_dummy() {
            true => 0,
            false => old_records[i].payload().balance,
        };

        let value_commitment_randomness = <ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

        let value_commitment = ValueCommitment::commit(
            &circuit_parameters.value_commitment,
            &value.to_le_bytes(),
            &value_commitment_randomness,
        )
        .unwrap();

        let proof = PredicateSNARK::prove(
            &pred_nizk_pp.proving_key,
            PaymentCircuit::new(
                &circuit_parameters,
                &local_data_comm,
                &value_commitment_randomness,
                &value_commitment,
                i as u8,
                value,
            ),
            &mut rng,
        )
        .expect("Proof should work");
        #[cfg(debug_assertions)]
        {
            let pred_pub_input: PaymentPredicateLocalData<Components> = PaymentPredicateLocalData {
                local_data_commitment_parameters: circuit_parameters
                    .local_data_commitment
                    .parameters()
                    .clone(),
                local_data_commitment: local_data_comm.clone(),
                value_commitment_parameters: circuit_parameters.value_commitment.parameters().clone(),
                value_commitment_randomness: value_commitment_randomness.clone(),
                value_commitment: value_commitment.clone(),
                position: i as u8,
            };
            assert!(PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify"));
        }
        let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
            verification_key: pred_nizk_pp.verification_key.clone(),
            proof,
            value_commitment,
            value_commitment_randomness,
        };
        old_proof_and_vk.push(private_input);
    }

    let mut new_proof_and_vk = vec![];
    for j in 0..NUM_OUTPUT_RECORDS {
        // If the record is a dummy, then the value should be 0
        let value = match new_records[j].is_dummy() {
            true => 0,
            false => new_records[j].payload().balance,
        };

        let value_commitment_randomness = <ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

        let value_commitment = ValueCommitment::commit(
            &circuit_parameters.value_commitment,
            &value.to_le_bytes(),
            &value_commitment_randomness,
        )
        .unwrap();

        let proof = PredicateSNARK::prove(
            &pred_nizk_pp.proving_key,
            PaymentCircuit::new(
                &circuit_parameters,
                &local_data_comm,
                &value_commitment_randomness,
                &value_commitment,
                j as u8,
                value,
            ),
            &mut rng,
        )
        .expect("Proof should work");

        #[cfg(debug_assertions)]
        {
            let pred_pub_input: PaymentPredicateLocalData<Components> = PaymentPredicateLocalData {
                local_data_commitment_parameters: circuit_parameters
                    .local_data_commitment
                    .parameters()
                    .clone(),
                local_data_commitment: local_data_comm.clone(),
                value_commitment_parameters: circuit_parameters.value_commitment.parameters().clone(),
                value_commitment_randomness: value_commitment_randomness.clone(),
                value_commitment: value_commitment.clone(),
                position: j as u8,
            };
            assert!(PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify"));
        }

        let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
            verification_key: pred_nizk_pp.verification_key.clone(),
            proof,
            value_commitment,
            value_commitment_randomness,
        };
        new_proof_and_vk.push(private_input);
    }

    // Generate the binding signature

    let mut old_value_commits = vec![];
    let mut old_value_commit_randomness = vec![];
    let mut new_value_commits = vec![];
    let mut new_value_commit_randomness = vec![];

    for death_pred_attr in &old_proof_and_vk {
        let mut commitment = [0u8; 32];
        let mut randomness = [0u8; 32];

        death_pred_attr.value_commitment.write(&mut commitment[..]).unwrap();
        death_pred_attr
            .value_commitment_randomness
            .write(&mut randomness[..])
            .unwrap();

        old_value_commits.push(commitment);
        old_value_commit_randomness.push(randomness);
    }

    for birth_pred_attr in &new_proof_and_vk {
        let mut commitment = [0u8; 32];
        let mut randomness = [0u8; 32];

        birth_pred_attr.value_commitment.write(&mut commitment[..]).unwrap();
        birth_pred_attr
            .value_commitment_randomness
            .write(&mut randomness[..])
            .unwrap();

        new_value_commits.push(commitment);
        new_value_commit_randomness.push(randomness);
    }

    let sighash = to_bytes![local_data_comm].unwrap();

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
        &local_data_comm,
        &local_data_rand,
        &memo,
        &auxiliary,
        &old_value_commits,
        &new_value_commits,
        value_balance,
        &binding_signature,
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

    // Check that the proof check constraint system was satisfied.
    let mut pf_check_cs = TestConstraintSystem::<Fq>::new();

    execute_outer_proof_gadget::<_, _>(
        &mut pf_check_cs.ns(|| "Check predicate proofs"),
        &circuit_parameters,
        &old_proof_and_vk,
        &new_proof_and_vk,
        &predicate_comm,
        &predicate_rand,
        &local_data_comm,
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

    let path = ledger.storage.storage.path().to_owned();
    drop(ledger);
    MerkleTreeLedger::destroy_storage(path).unwrap();
}
