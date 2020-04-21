use crate::{
    address::{AddressPair, AddressPublicKey},
    base_dpc::{
        instantiated::*,
        predicate::DPCPredicate,
        record::DPCRecord,
        record_payload::PaymentRecordPayload,
        BaseDPCComponents,
        DPC,
    },
    consensus::ConsensusParameters,
    DPCScheme,
};

use snarkos_models::{algorithms::CRH, dpc::Record};
use snarkos_objects::{
    dpc::{transactions::DPCTransactions, Block},
    ledger::Ledger,
    merkle_root,
    BlockHeader,
    MerkleRootHash,
};
use snarkos_utilities::{bytes::ToBytes, storage::Storage, to_bytes};

use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TEST_CONSENSUS: ConsensusParameters = ConsensusParameters {
    max_block_size: 1_000_000_000usize,
    max_nonce: u32::max_value(),
    target_block_time: 10i64, //unix seconds
};

pub struct Wallet {
    pub secret_key: &'static str,
    pub public_key: &'static str,
}

pub fn setup_or_load_parameters<R: Rng>(
    rng: &mut R,
) -> (
    <Components as BaseDPCComponents>::MerkleParameters,
    <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
) {
    let mut path = std::env::current_dir().unwrap();
    path.push("src/parameters/");
    let ledger_parameter_path = path.join("ledger.params");

    let (ledger_parameters, parameters) =
        match <Components as BaseDPCComponents>::MerkleParameters::load(&ledger_parameter_path) {
            Ok(ledger_parameters) => {
                let parameters = match <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters::load(&path) {
                    Ok(parameters) => parameters,
                    Err(_) => {
                        println!("Parameter Setup");
                        <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::setup(&ledger_parameters, rng)
                            .expect("DPC setup failed")
                    }
                };

                (ledger_parameters, parameters)
            }
            Err(_) => {
                println!("Ledger parameter Setup");
                let ledger_parameters = MerkleTreeLedger::setup(rng).expect("Ledger setup failed");

                println!("Parameter Setup");
                let parameters = <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::setup(&ledger_parameters, rng)
                    .expect("DPC setup failed");

                (ledger_parameters, parameters)
            }
        };

    // Store parameters
    //    ledger_parameters.store(&ledger_parameter_path).unwrap();
    //    parameters.store(&path).unwrap();

    (ledger_parameters, parameters)
}

pub fn generate_test_addresses<R: Rng>(
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    rng: &mut R,
) -> [AddressPair<Components>; 3] {
    let genesis_metadata = [1u8; 32];
    let genesis_address = DPC::create_address_helper(&parameters.circuit_parameters, &genesis_metadata, rng).unwrap();

    let metadata_1 = [2u8; 32];
    let address_1 = DPC::create_address_helper(&parameters.circuit_parameters, &metadata_1, rng).unwrap();

    let metadata_2 = [3u8; 32];
    let address_2 = DPC::create_address_helper(&parameters.circuit_parameters, &metadata_2, rng).unwrap();

    // TODO Setup permanent test addresses. Note: addresses must be regenerated if circuit parameters change
    [genesis_address, address_1, address_2]
}

pub fn setup_ledger<R: Rng>(
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    ledger_parameters: <Components as BaseDPCComponents>::MerkleParameters,
    genesis_address: &AddressPair<Components>,
    rng: &mut R,
) -> (MerkleTreeLedger, Vec<u8>) {
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
        rng,
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
    let ledger = MerkleTreeLedger::new(
        ledger_parameters,
        genesis_record.commitment(),
        genesis_sn.clone(),
        genesis_memo,
    )
    .unwrap();

    (ledger, genesis_pred_vk_bytes)
}

pub fn create_block_with_coinbase_transaction<R: Rng>(
    transactions: &mut DPCTransactions<Tx>,
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    genesis_pred_vk_bytes: &Vec<u8>,
    new_birth_predicates: Vec<DPCPredicate<Components>>,
    new_death_predicates: Vec<DPCPredicate<Components>>,
    genesis_address: AddressPair<Components>,
    recipient: AddressPublicKey<Components>,
    consensus: &ConsensusParameters,
    ledger: &MerkleTreeLedger,
    rng: &mut R,
) -> (Vec<DPCRecord<Components>>, Block<Tx>) {
    let (new_coinbase_records, transaction) = ConsensusParameters::create_coinbase_transaction(
        ledger.len() as u32,
        &transactions,
        &parameters,
        &genesis_pred_vk_bytes,
        new_birth_predicates,
        new_death_predicates,
        genesis_address,
        recipient,
        &ledger,
        rng,
    )
    .unwrap();

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

    let previous_block = ledger.get_latest_block().unwrap();

    // Pseudo mining
    let header = BlockHeader {
        previous_block_hash: previous_block.header.get_hash(),
        merkle_root_hash: MerkleRootHash(merkle_root_bytes),
        time,
        difficulty_target: consensus.get_block_difficulty(&previous_block.header, time),
        nonce: 0,
    };

    let block = Block {
        header,
        transactions: transactions.clone(),
    };

    (new_coinbase_records, block)
}
