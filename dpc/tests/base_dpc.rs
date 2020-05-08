#[cfg(debug_assertions)]
use snarkos_algorithms::snark::PreparedVerifyingKey;
use snarkos_dpc::{
    base_dpc::{
        instantiated::*,
        payment_circuit::*,
        predicate::PrivatePredicateInput,
        record_payload::PaymentRecordPayload,
        BaseDPCComponents,
        LocalData,
        DPC,
    },
    test_data::*,
    DPCScheme,
};
use snarkos_models::{
    algorithms::{CommitmentScheme, CRH, SNARK},
    dpc::Record,
};
use snarkos_objects::{
    dpc::{Block, DPCTransactions},
    ledger::Ledger,
    merkle_root,
    BlockHeader,
    MerkleRootHash,
};
use snarkos_storage::BlockStorage;
use snarkos_utilities::rand::UniformRand;

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn base_dpc_integration_test() {
    let mut rng = XorShiftRng::seed_from_u64(23472342u64);

    // Generate or load parameters for the ledger, commitment schemes, and CRH
    let (ledger_parameters, parameters) = setup_or_load_parameters(false, &mut rng);

    // Generate addresses
    let [genesis_address, recipient, _] = generate_test_addresses(&parameters, &mut rng);

    // Setup the ledger
    let (ledger, genesis_pred_vk_bytes) = setup_ledger(
        "test_dpc_integration_db".to_string(),
        &parameters,
        ledger_parameters,
        &genesis_address,
        &mut rng,
    );

    #[cfg(debug_assertions)]
    let pred_nizk_pvk: PreparedVerifyingKey<_> = parameters.predicate_snark_parameters.verification_key.clone().into();

    // Generate dummy input records having as address the genesis address.
    let old_account_private_keys = vec![genesis_address.private_key.clone(); NUM_INPUT_RECORDS];
    let mut old_records = vec![];
    for i in 0..NUM_INPUT_RECORDS {
        let old_sn_nonce = SerialNumberNonce::hash(
            &parameters.circuit_parameters.serial_number_nonce_parameters,
            &[64u8 + (i as u8); 1],
        )
        .unwrap();
        let old_record = DPC::generate_record(
            &parameters.circuit_parameters,
            &old_sn_nonce,
            &genesis_address.public_key,
            true, // The input record is dummy
            &PaymentRecordPayload::default(),
            &Predicate::new(genesis_pred_vk_bytes.clone()),
            &Predicate::new(genesis_pred_vk_bytes.clone()),
            &mut rng,
        )
        .unwrap();
        old_records.push(old_record);
    }

    // Construct new records.

    // Create a payload.
    let new_payload = PaymentRecordPayload { balance: 10, lock: 0 };

    // Set the new records' predicate to be the "always-accept" predicate.
    let new_predicate = Predicate::new(genesis_pred_vk_bytes.clone());

    let new_account_public_keys = vec![recipient.public_key.clone(); NUM_OUTPUT_RECORDS];
    let new_payloads = vec![new_payload.clone(); NUM_OUTPUT_RECORDS];
    let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let new_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];

    let auxiliary = [3u8; 32];
    let memo = [4u8; 32];

    let old_death_vk_and_proof_generator = |local_data: &LocalData<Components>| {
        let mut rng = XorShiftRng::seed_from_u64(23472342u64);
        let mut old_proof_and_vk = vec![];
        for i in 0..NUM_INPUT_RECORDS {
            // If the record is a dummy, then the value should be 0
            let input_value = match local_data.old_records[i].is_dummy() {
                true => 0,
                false => local_data.old_records[i].payload().balance,
            };

            // Generate value commitment randomness
            let value_commitment_randomness =
                <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

            // Generate the value commitment
            let value_commitment = local_data
                .circuit_parameters
                .value_commitment_parameters
                .commit(&input_value.to_le_bytes(), &value_commitment_randomness)
                .unwrap();

            // Instantiate death predicate circuit
            let death_predicate_circuit = PaymentCircuit::new(
                &local_data.circuit_parameters,
                &local_data.local_data_commitment,
                &value_commitment_randomness,
                &value_commitment,
                i as u8,
                input_value,
            );

            // Generate the predicate proof
            let proof = PredicateSNARK::prove(
                &parameters.predicate_snark_parameters.proving_key,
                death_predicate_circuit,
                &mut rng,
            )
            .expect("Proving should work");
            #[cfg(debug_assertions)]
            {
                let pred_pub_input: PaymentPredicateLocalData<Components> = PaymentPredicateLocalData {
                    local_data_commitment_parameters: local_data
                        .circuit_parameters
                        .local_data_commitment_parameters
                        .parameters()
                        .clone(),
                    local_data_commitment: local_data.local_data_commitment.clone(),
                    value_commitment_parameters: local_data
                        .circuit_parameters
                        .value_commitment_parameters
                        .parameters()
                        .clone(),
                    value_commitment_randomness: value_commitment_randomness.clone(),
                    value_commitment: value_commitment.clone(),
                    position: i as u8,
                };
                assert!(PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify"));
            }

            let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                proof,
                value_commitment,
                value_commitment_randomness,
            };
            old_proof_and_vk.push(private_input);
        }
        Ok(old_proof_and_vk)
    };
    let new_birth_vk_and_proof_generator = |local_data: &LocalData<Components>| {
        let mut rng = XorShiftRng::seed_from_u64(23472342u64);
        let mut new_proof_and_vk = vec![];
        for j in 0..NUM_OUTPUT_RECORDS {
            // If the record is a dummy, then the value should be 0
            let output_value = match local_data.new_records[j].is_dummy() {
                true => 0,
                false => local_data.new_records[j].payload().balance,
            };

            // Generate value commitment randomness
            let value_commitment_randomness =
                <<Components as BaseDPCComponents>::ValueCommitment as CommitmentScheme>::Randomness::rand(&mut rng);

            // Generate the value commitment
            let value_commitment = local_data
                .circuit_parameters
                .value_commitment_parameters
                .commit(&output_value.to_le_bytes(), &value_commitment_randomness)
                .unwrap();

            // Instantiate birth predicate circuit
            let birth_predicate_circuit = PaymentCircuit::new(
                &local_data.circuit_parameters,
                &local_data.local_data_commitment,
                &value_commitment_randomness,
                &value_commitment,
                j as u8,
                output_value,
            );

            // Generate the predicate proof
            let proof = PredicateSNARK::prove(
                &parameters.predicate_snark_parameters.proving_key,
                birth_predicate_circuit,
                &mut rng,
            )
            .expect("Proving should work");
            #[cfg(debug_assertions)]
            {
                let pred_pub_input: PaymentPredicateLocalData<Components> = PaymentPredicateLocalData {
                    local_data_commitment_parameters: local_data
                        .circuit_parameters
                        .local_data_commitment_parameters
                        .parameters()
                        .clone(),
                    local_data_commitment: local_data.local_data_commitment.clone(),
                    value_commitment_parameters: local_data
                        .circuit_parameters
                        .value_commitment_parameters
                        .parameters()
                        .clone(),
                    value_commitment_randomness: value_commitment_randomness.clone(),
                    value_commitment: value_commitment.clone(),
                    position: j as u8,
                };
                assert!(PredicateSNARK::verify(&pred_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify"));
            }
            let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                proof,
                value_commitment,
                value_commitment_randomness,
            };
            new_proof_and_vk.push(private_input);
        }
        Ok(new_proof_and_vk)
    };

    let (_new_records, transaction) = InstantiatedDPC::execute(
        &parameters,
        &old_records,
        &old_account_private_keys,
        &old_death_vk_and_proof_generator,
        &new_account_public_keys,
        &new_dummy_flags,
        &new_payloads,
        &new_birth_predicates,
        &new_death_predicates,
        &new_birth_vk_and_proof_generator,
        &auxiliary,
        &memo,
        &ledger,
        &mut rng,
    )
    .unwrap();

    // Craft the block

    let previous_block = ledger.get_latest_block().unwrap();

    let mut transactions = DPCTransactions::new();
    transactions.push(transaction);

    let transaction_ids: Vec<Vec<u8>> = transactions
        .to_transaction_ids()
        .unwrap()
        .iter()
        .map(|id| id.to_vec())
        .collect();

    let mut merkle_root_bytes = [0u8; 32];
    merkle_root_bytes[..].copy_from_slice(&merkle_root(&transaction_ids));

    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64;

    let header = BlockHeader {
        previous_block_hash: previous_block.header.get_hash(),
        merkle_root_hash: MerkleRootHash(merkle_root_bytes),
        time,
        difficulty_target: previous_block.header.difficulty_target,
        nonce: 0,
    };

    assert!(InstantiatedDPC::verify_transactions(&parameters, &transactions.0, &ledger).unwrap());

    let block = Block { header, transactions };

    ledger.insert_block(&block).unwrap();
    assert_eq!(ledger.len(), 2);

    let path = ledger.storage.storage.path().to_owned();
    drop(ledger);
    BlockStorage::<Tx, <Components as BaseDPCComponents>::MerkleParameters>::destroy_storage(path).unwrap();
}
