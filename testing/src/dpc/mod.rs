use snarkos_consensus::MerkleTreeLedger;
use snarkos_dpc::base_dpc::{instantiated::*, parameters::PublicParameters};
use snarkos_models::{
    algorithms::{MerkleParameters, CRH},
    dpc::DPCScheme,
    objects::AccountScheme,
    parameters::Parameters,
};
use snarkos_objects::Account;
use snarkos_parameters::LedgerMerkleTreeParameters;
use snarkos_utilities::bytes::FromBytes;

use rand::Rng;

pub fn setup_or_load_parameters<R: Rng>(
    verify_only: bool,
    rng: &mut R,
) -> (
    CommitmentMerkleParameters,
    <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
) {
    // TODO (howardwu): Resolve this inconsistency on import structure with a new model once MerkleParameters are refactored.
    let crh_parameters =
        <MerkleTreeCRH as CRH>::Parameters::read(&LedgerMerkleTreeParameters::load_bytes().unwrap()[..])
            .expect("read bytes as hash for MerkleParameters in ledger");
    let merkle_tree_hash_parameters = <CommitmentMerkleParameters as MerkleParameters>::H::from(crh_parameters);
    let ledger_merkle_tree_parameters = From::from(merkle_tree_hash_parameters);

    let parameters = match <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters::load(verify_only) {
        Ok(parameters) => parameters,
        Err(err) => {
            println!("error - {}, re-running parameter Setup", err);
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

    let genesis_account = Account::new(signature_parameters, commitment_parameters, rng).unwrap();
    let account_1 = Account::new(signature_parameters, commitment_parameters, rng).unwrap();
    let account_2 = Account::new(signature_parameters, commitment_parameters, rng).unwrap();

    [genesis_account, account_1, account_2]
}
