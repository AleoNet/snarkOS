use crate::{
    address::AddressPair,
    base_dpc::{instantiated::*, record_payload::PaymentRecordPayload, BaseDPCComponents, DPC},
    DPCScheme,
};

use snarkos_models::{algorithms::CRH, dpc::Record, storage::Storage};
use snarkos_objects::ledger::Ledger;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use rand::Rng;

pub struct Wallet {
    pub secret_key: &'static str,
    pub public_key: &'static str,
}

pub fn setup_or_load_parameters<R: Rng>(
    verify_only: bool,
    rng: &mut R,
) -> (
    <Components as BaseDPCComponents>::MerkleParameters,
    <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
) {
    let mut path = std::env::current_dir().unwrap();
    path.push("../dpc/src/parameters/");
    let ledger_parameter_path = path.join("ledger.params");

    let (ledger_parameters, parameters) =
        match <Components as BaseDPCComponents>::MerkleParameters::load(&ledger_parameter_path) {
            Ok(ledger_parameters) => {
                let parameters =
                    match <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters::load(&path, verify_only) {
                        Ok(parameters) => parameters,
                        Err(_) => {
                            println!("Parameter Setup");
                            let parameters =
                                <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::setup(&ledger_parameters, rng)
                                    .expect("DPC setup failed");

                            //  parameters.store(&path).unwrap();
                            parameters
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

                //  ledger_parameters.store(&ledger_parameter_path).unwrap();
                //  parameters.store(&path).unwrap();

                (ledger_parameters, parameters)
            }
        };

    // Store parameters - Uncomment this to store parameters to the specified paths
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

    [genesis_address, address_1, address_2]
}

pub fn setup_ledger<R: Rng>(
    db_name: String,
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
    let genesis_memo = [0u8; 32];

    let mut path = std::env::current_dir().unwrap();
    path.push(db_name);

    // Use genesis record, serial number, and memo to initialize the ledger.
    let ledger = MerkleTreeLedger::new(
        &path,
        ledger_parameters,
        genesis_record.commitment(),
        genesis_sn.clone(),
        genesis_memo,
        genesis_pred_vk_bytes.to_vec(),
        to_bytes![genesis_address].unwrap().to_vec(),
    )
    .unwrap();

    (ledger, genesis_pred_vk_bytes)
}
