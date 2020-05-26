use crate::base_dpc::{instantiated::*, parameters::PublicParameters, record_payload::PaymentRecordPayload, DPC};
use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_models::{
    algorithms::CRH,
    dpc::{DPCScheme, Record},
    objects::{AccountScheme, Transaction},
    parameters::Parameter,
};
use snarkos_objects::Account;
use snarkos_parameters::LedgerMerkleTreeParameters;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::Rng;

pub struct Wallet {
    pub private_key: &'static str,
    pub public_key: &'static str,
}

/// Attributes used to instantiate a new ledger
pub struct GenesisAttributes {
    pub genesis_cm: <Tx as Transaction>::Commitment,
    pub genesis_sn: <Tx as Transaction>::SerialNumber,
    pub genesis_memo: <Tx as Transaction>::Memorandum,
    pub genesis_pred_vk_bytes: Vec<u8>,
    pub genesis_account_bytes: Vec<u8>,
}

pub fn setup_or_load_parameters<R: Rng>(
    verify_only: bool,
    rng: &mut R,
) -> (
    CommitmentMerkleParameters,
    <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
) {
    // TODO (howardwu): Resolve this inconsistency on import structure with a new model once MerkleParameters are refactored.
    let crh_parameters = <MerkleTreeCRH as CRH>::Parameters::read(&LedgerMerkleTreeParameters::load_bytes()[..])
        .expect("read bytes as hash for MerkleParameters in ledger");
    let merkle_tree_hash_parameters = <CommitmentMerkleParameters as MerkleParameters>::H::from(crh_parameters);
    let ledger_merkle_tree_parameters = From::from(merkle_tree_hash_parameters);

    // TODO (howardwu): Remove this hardcoded path.
    let mut path = std::env::current_dir().unwrap();
    path.push("../dpc/src/parameters/");
    let parameters = match <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters::load(&path, verify_only) {
        Ok(parameters) => parameters,
        Err(err) => {
            println!("Err: {}. Path: {:?}. Re-running parameter Setup", err, path);
            <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::setup(&ledger_merkle_tree_parameters, rng)
                .expect("DPC setup failed")
        }
    };

    (ledger_merkle_tree_parameters, parameters)
}

pub fn load_verifying_parameters() -> PublicParameters<Components> {
    PublicParameters::<Components>::load_vk_direct().unwrap()
}

pub fn generate_test_accounts<R: Rng>(
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    rng: &mut R,
) -> [Account<Components>; 3] {
    let signature_parameters = &parameters.circuit_parameters.account_signature;
    let commitment_parameters = &parameters.circuit_parameters.account_commitment;

    let genesis_metadata = [1u8; 32];
    let genesis_account = Account::new(signature_parameters, commitment_parameters, &genesis_metadata, rng).unwrap();

    let metadata_1 = [2u8; 32];
    let account_1 = Account::new(signature_parameters, commitment_parameters, &metadata_1, rng).unwrap();

    let metadata_2 = [3u8; 32];
    let account_2 = Account::new(signature_parameters, commitment_parameters, &metadata_2, rng).unwrap();

    [genesis_account, account_1, account_2]
}

pub fn ledger_genesis_setup<R: Rng>(
    parameters: &<InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    genesis_account: &Account<Components>,
    rng: &mut R,
) -> GenesisAttributes {
    let genesis_sn_nonce =
        SerialNumberNonce::hash(&parameters.circuit_parameters.serial_number_nonce, &[34u8; 1]).unwrap();
    let genesis_predicate_vk_bytes = to_bytes![
        PredicateVerificationKeyHash::hash(
            &parameters.circuit_parameters.predicate_verification_key_hash,
            &to_bytes![parameters.predicate_snark_parameters.verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let genesis_record = DPC::generate_record(
        &parameters.circuit_parameters,
        &genesis_sn_nonce,
        &genesis_account.public_key,
        true, // The inital record should be dummy
        &PaymentRecordPayload::default(),
        &Predicate::new(genesis_predicate_vk_bytes.clone()),
        &Predicate::new(genesis_predicate_vk_bytes.clone()),
        rng,
    )
    .unwrap();

    // Generate serial number for the genesis record.
    let (genesis_sn, _) = DPC::generate_sn(
        &parameters.circuit_parameters,
        &genesis_record,
        &genesis_account.private_key,
    )
    .unwrap();
    let genesis_memo = [0u8; 32];

    GenesisAttributes {
        genesis_cm: genesis_record.commitment(),
        genesis_sn,
        genesis_memo,
        genesis_pred_vk_bytes: genesis_predicate_vk_bytes.to_vec(),
        genesis_account_bytes: to_bytes![genesis_account].unwrap(),
    }
}
