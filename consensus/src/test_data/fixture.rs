use snarkos_dpc::{
    base_dpc::instantiated::*,
    test_data::{generate_test_accounts, setup_ledger, setup_or_load_parameters},
};
use snarkos_models::dpc::DPCScheme;
use snarkos_objects::Account;

use once_cell::sync::Lazy;
use rand::{Rng, SeedableRng};
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

    let mut path = std::env::temp_dir();
    let random_storage_path: usize = rng.gen();
    path.push(format!("test_db_{}", random_storage_path));

    let (ledger, genesis_pred_vk_bytes) = setup_ledger(
        &path,
        &parameters,
        ledger_parameters.clone(),
        &test_accounts[0],
        &mut rng,
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
