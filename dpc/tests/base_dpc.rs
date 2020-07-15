#[cfg(debug_assertions)]
use snarkos_algorithms::snark::gm17::PreparedVerifyingKey;
use snarkos_dpc::{
    base_dpc::{
        instantiated::*,
        predicate::PrivatePredicateInput,
        predicate_circuit::*,
        record_payload::RecordPayload,
        records::record_serializer::*,
        LocalData,
        DPC,
    },
    dpc::base_dpc::BaseDPCComponents,
};
use snarkos_models::{
    algorithms::{CommitmentScheme, EncryptionScheme, CRH, SNARK},
    dpc::{DPCScheme, Record},
    objects::{LedgerScheme, Transaction},
};
use snarkos_objects::{
    dpc::DPCTransactions,
    merkle_root,
    AccountViewKey,
    Block,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
    PedersenMerkleRootHash,
    ProofOfSuccinctWork,
};
use snarkos_testing::{dpc::*, storage::*};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn base_dpc_integration_test() {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    // Generate or load parameters for the ledger, commitment schemes, and CRH
    let (ledger_parameters, parameters) = setup_or_load_parameters(false, &mut rng);

    // Generate accounts
    let [genesis_account, recipient, _] = generate_test_accounts(&parameters, &mut rng);

    // Specify network_id
    let network_id: u8 = 0;

    // Create a genesis block

    let genesis_block = Block {
        header: BlockHeader {
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            merkle_root_hash: MerkleRootHash([0u8; 32]),
            pedersen_merkle_root_hash: PedersenMerkleRootHash([0u8; 32]),
            time: 0,
            difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
            nonce: 0,
            proof: ProofOfSuccinctWork::default(),
        },
        transactions: DPCTransactions::new(),
    };

    let ledger = initialize_test_blockchain::<Tx, CommitmentMerkleParameters>(ledger_parameters, genesis_block);

    let predicate_vk_hash = to_bytes![
        PredicateVerificationKeyHash::hash(
            &parameters.circuit_parameters.predicate_verification_key_hash,
            &to_bytes![parameters.predicate_snark_parameters().verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    #[cfg(debug_assertions)]
    let predicate_nizk_pvk: PreparedVerifyingKey<_> =
        parameters.predicate_snark_parameters.verification_key.clone().into();

    // Generate dummy input records having as address the genesis address.
    let old_account_private_keys = vec![genesis_account.private_key.clone(); NUM_INPUT_RECORDS];
    let mut old_records = vec![];
    for i in 0..NUM_INPUT_RECORDS {
        let old_sn_nonce = SerialNumberNonce::hash(
            &parameters.circuit_parameters.serial_number_nonce,
            &[64u8 + (i as u8); 1],
        )
        .unwrap();
        let old_record = DPC::generate_record(
            &parameters.circuit_parameters,
            &old_sn_nonce,
            &genesis_account.address,
            true, // The input record is dummy
            0,
            &RecordPayload::default(),
            &Predicate::new(predicate_vk_hash.clone()),
            &Predicate::new(predicate_vk_hash.clone()),
            &mut rng,
        )
        .unwrap();
        old_records.push(old_record);
    }

    // Construct new records.

    // Set the new records' predicate to be the "always-accept" predicate.
    let new_predicate = Predicate::new(predicate_vk_hash.clone());

    let new_account_addresss = vec![recipient.address.clone(); NUM_OUTPUT_RECORDS];
    let new_dummy_flags = vec![false; NUM_OUTPUT_RECORDS];
    let new_values = vec![10; NUM_OUTPUT_RECORDS];
    let new_payloads = vec![RecordPayload::default(); NUM_OUTPUT_RECORDS];
    let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
    let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];

    let memo = [4u8; 32];

    let old_death_vk_and_proof_generator = |local_data: &LocalData<Components>| {
        let mut rng = XorShiftRng::seed_from_u64(23472342u64);
        let mut old_proof_and_vk = vec![];
        for i in 0..NUM_INPUT_RECORDS {
            // Instantiate death predicate circuit
            let death_predicate_circuit = PredicateCircuit::new(
                &local_data.circuit_parameters,
                &local_data.local_data_commitment,
                i as u8,
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
                let pred_pub_input: PredicateLocalData<Components> = PredicateLocalData {
                    local_data_commitment_parameters: local_data
                        .circuit_parameters
                        .local_data_commitment
                        .parameters()
                        .clone(),
                    local_data_commitment: local_data.local_data_commitment.clone(),
                    position: i as u8,
                };
                assert!(
                    PredicateSNARK::verify(&predicate_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify")
                );
            }

            let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                proof,
            };
            old_proof_and_vk.push(private_input);
        }
        Ok(old_proof_and_vk)
    };
    let new_birth_vk_and_proof_generator = |local_data: &LocalData<Components>| {
        let mut rng = XorShiftRng::seed_from_u64(23472342u64);
        let mut new_proof_and_vk = vec![];
        for j in 0..NUM_OUTPUT_RECORDS {
            // Instantiate birth predicate circuit
            let birth_predicate_circuit = PredicateCircuit::new(
                &local_data.circuit_parameters,
                &local_data.local_data_commitment,
                j as u8,
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
                let pred_pub_input: PredicateLocalData<Components> = PredicateLocalData {
                    local_data_commitment_parameters: local_data
                        .circuit_parameters
                        .local_data_commitment
                        .parameters()
                        .clone(),
                    local_data_commitment: local_data.local_data_commitment.clone(),
                    position: j as u8,
                };
                assert!(
                    PredicateSNARK::verify(&predicate_nizk_pvk, &pred_pub_input, &proof).expect("Proof should verify")
                );
            }
            let private_input: PrivatePredicateInput<Components> = PrivatePredicateInput {
                verification_key: parameters.predicate_snark_parameters.verification_key.clone(),
                proof,
            };
            new_proof_and_vk.push(private_input);
        }
        Ok(new_proof_and_vk)
    };

    let (new_records, transaction) = InstantiatedDPC::execute(
        &parameters,
        &old_records,
        &old_account_private_keys,
        &old_death_vk_and_proof_generator,
        &new_account_addresss,
        &new_dummy_flags,
        &new_values,
        &new_payloads,
        &new_birth_predicates,
        &new_death_predicates,
        &new_birth_vk_and_proof_generator,
        &memo,
        network_id,
        &ledger,
        &mut rng,
    )
    .unwrap();

    let transaction_bytes = to_bytes![transaction].unwrap();
    let _recovered_transaction = Tx::read(&transaction_bytes[..]).unwrap();

    {
        // Check that new_records can be decrypted from the transaction

        let record_ciphertexts = transaction.ciphertexts();
        let new_account_private_keys = vec![recipient.private_key.clone(); NUM_OUTPUT_RECORDS];

        for (((ciphertext, private_key), new_record), selector_bits) in record_ciphertexts
            .iter()
            .zip(new_account_private_keys)
            .zip(new_records)
            .zip(&transaction.new_records_ciphertext_and_fq_high_selectors)
        {
            let final_fq_high_bit = selector_bits.1.clone();

            let view_key = AccountViewKey::from_private_key(
                &parameters.circuit_parameters.account_signature,
                &parameters.circuit_parameters.account_commitment,
                &private_key,
            )
            .unwrap();

            let plaintext = parameters
                .circuit_parameters
                .account_encryption
                .decrypt(&view_key.decryption_key, &ciphertext)
                .unwrap();

            let record_components = RecordSerializer::<
                Components,
                <Components as BaseDPCComponents>::EncryptionModelParameters,
                <Components as BaseDPCComponents>::EncryptionGroup,
            >::deserialize(plaintext, final_fq_high_bit)
            .unwrap();

            assert_eq!(record_components.value, new_record.value());
            assert_eq!(record_components.payload, *new_record.payload());
            assert_eq!(
                record_components.birth_predicate_repr,
                new_record.birth_predicate_repr().to_vec()
            );
            assert_eq!(
                record_components.death_predicate_repr,
                new_record.death_predicate_repr().to_vec()
            );
            assert_eq!(&record_components.serial_number_nonce, new_record.serial_number_nonce());
            assert_eq!(
                record_components.commitment_randomness,
                new_record.commitment_randomness()
            );
        }
    }

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
        pedersen_merkle_root_hash: PedersenMerkleRootHash([0u8; 32]),
        proof: ProofOfSuccinctWork::default(),
    };

    assert!(InstantiatedDPC::verify_transactions(&parameters, &transactions.0, &ledger).unwrap());

    let block = Block { header, transactions };

    ledger.insert_and_commit(&block).unwrap();
    assert_eq!(ledger.len(), 2);

    kill_storage(ledger);
}
