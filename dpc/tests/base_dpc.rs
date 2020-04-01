#[cfg(debug_assertions)]
use snarkos_algorithms::{merkle_tree::MerkleParameters, snark::PreparedVerifyingKey};
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
    ledger::{transactions::Transactions, Block, Ledger},
    DPCScheme,
    Record,
};
use snarkos_models::algorithms::{CommitmentScheme, CRH, SNARK};
use snarkos_objects::{merkle_root, BlockHeader, MerkleRootHash};
use snarkos_utilities::{bytes::ToBytes, rand::UniformRand, storage::Storage, to_bytes};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn base_dpc_integration_test() {
    let mut rng = XorShiftRng::seed_from_u64(23472342u64);

    let mut path = std::env::current_dir().unwrap();
    path.push("src/parameters/");
    let ledger_parameter_path = path.join("ledger.params");

    // Generate or load parameters for the ledger, commitment schemes, CRH, and the
    // "always-accept" predicate.
    let (ledger_parameters, parameters) =
        match <<Components as BaseDPCComponents>::MerkleParameters as MerkleParameters>::H::load(&ledger_parameter_path)
        {
            Ok(ledger_parameters) => {
                let parameters = match <InstantiatedDPC as DPCScheme<MerkleTreeIdealLedger>>::Parameters::load(&path) {
                    Ok(parameters) => parameters,
                    Err(_) => {
                        println!("Parameter Setup");
                        <InstantiatedDPC as DPCScheme<MerkleTreeIdealLedger>>::setup(&ledger_parameters, &mut rng)
                            .expect("DPC setup failed")
                    }
                };

                (ledger_parameters, parameters)
            }
            Err(_) => {
                println!("Ledger parameter Setup");
                let ledger_parameters = MerkleTreeIdealLedger::setup(&mut rng).expect("Ledger setup failed");

                println!("Parameter Setup");
                let parameters =
                    <InstantiatedDPC as DPCScheme<MerkleTreeIdealLedger>>::setup(&ledger_parameters, &mut rng)
                        .expect("DPC setup failed");

                (ledger_parameters, parameters)
            }
        };

    // Store parameters
    //    ledger_parameters.store(&ledger_parameter_path).unwrap();
    //    parameters.store(&path).unwrap();

    #[cfg(debug_assertions)]
    let pred_nizk_pvk: PreparedVerifyingKey<_> = parameters.predicate_snark_parameters.verification_key.clone().into();
    // Generate metadata and an address for a dummy initial, or "genesis", record.
    let genesis_metadata = [1u8; 32];
    let genesis_address =
        DPC::create_address_helper(&parameters.circuit_parameters, &genesis_metadata, &mut rng).unwrap();

    let genesis_sn_nonce = SerialNumberNonce::hash(
        &parameters.circuit_parameters.serial_number_nonce_parameters,
        &[34u8; 1],
    )
    .unwrap();
    let genesis_pred_vk_bytes = to_bytes![
        PredicateVerificationKeyHash::hash(
            &parameters.circuit_parameters.predicate_verification_key_hash_parameters,
            &to_bytes![parameters.predicate_snark_parameters.verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let genesis_record = DPC::generate_record(
        &parameters.circuit_parameters,
        &genesis_sn_nonce,
        &genesis_address.public_key,
        true, // The inital record should be dummy
        &PaymentRecordPayload::default(),
        &Predicate::new(genesis_pred_vk_bytes.clone()),
        &Predicate::new(genesis_pred_vk_bytes.clone()),
        &mut rng,
    )
    .unwrap();

    // Generate serial number for the genesis record.
    let (genesis_sn, _) = DPC::generate_sn(
        &parameters.circuit_parameters,
        &genesis_record,
        &genesis_address.secret_key,
    )
    .unwrap();
    let genesis_memo = [1u8; 32];

    // Use genesis record, serial number, and memo to initialize the ledger.
    let mut ledger = MerkleTreeIdealLedger::new(
        ledger_parameters,
        genesis_record.commitment(),
        genesis_sn.clone(),
        genesis_memo,
    );

    // Generate dummy input records having as address the genesis address.
    let old_asks = vec![genesis_address.secret_key.clone(); NUM_INPUT_RECORDS];
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

    // Create an address for an actual new record.
    let new_metadata = [2u8; 32];
    let new_address = DPC::create_address_helper(&parameters.circuit_parameters, &new_metadata, &mut rng).unwrap();

    // Create a payload.
    let new_payload = PaymentRecordPayload::default();
    //    let new_payload = PaymentRecordPayload {
    //        balance: 10,
    //        lock: 0,
    //    };

    // Set the new records' predicate to be the "always-accept" predicate.
    let new_predicate = Predicate::new(genesis_pred_vk_bytes.clone());

    let new_apks = vec![new_address.public_key.clone(); NUM_OUTPUT_RECORDS];
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
        old_proof_and_vk
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
        new_proof_and_vk
    };

    let (_new_records, transaction) = InstantiatedDPC::execute(
        &parameters,
        &old_records,
        &old_asks,
        &old_death_vk_and_proof_generator,
        &new_apks,
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

    let previous_block = ledger.blocks.last().unwrap();

    let mut transactions = Transactions::new();
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

    let block = Block { header, transactions };

    assert!(InstantiatedDPC::verify_block(&parameters, &block, &ledger).unwrap());

    ledger.push_block(block).unwrap();
    assert_eq!(ledger.len(), 2);
}
