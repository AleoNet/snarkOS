use snarkos_dpc::{
    base_dpc::instantiated::*,
    test_data::{generate_test_accounts, ledger_genesis_setup, setup_or_load_parameters},
};
use snarkos_models::dpc::DPCScheme;
use snarkos_objects::Account;
use snarkos_storage::test_data::*;

use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

pub static FIXTURE: Lazy<Fixture> = Lazy::new(|| setup());

// helper for setting up e2e tests
pub struct Fixture {
    pub parameters: <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    pub test_accounts: [Account<Components>; 3],
    pub ledger: MerkleTreeLedger,
    pub predicate: Predicate,
    pub rng: XorShiftRng,
}

fn setup() -> Fixture {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    // Generate or load parameters for the ledger, commitment schemes, and CRH
    let (ledger_parameters, parameters) = setup_or_load_parameters(false, &mut rng);

    // Generate addresses
    let test_accounts = generate_test_accounts(&parameters, &mut rng);

    let (genesis_cm, genesis_sn, genesis_memo, genesis_pred_vk_bytes, genesis_account_bytes) =
        ledger_genesis_setup(&parameters, &test_accounts[0], &mut rng);

    let ledger: MerkleTreeLedger = initialize_test_blockchain(
        ledger_parameters,
        genesis_cm,
        genesis_sn,
        genesis_memo,
        genesis_pred_vk_bytes.clone(),
        genesis_account_bytes,
    );

    let predicate = Predicate::new(genesis_pred_vk_bytes);

    Fixture {
        parameters,
        test_accounts,
        ledger,
        predicate,
        rng,
    }
}
