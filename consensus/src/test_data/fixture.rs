use snarkos_dpc::{
    base_dpc::instantiated::*,
    test_data::{generate_test_accounts, ledger_genesis_setup, setup_or_load_parameters, GenesisAttributes},
};
use snarkos_models::dpc::DPCScheme;
use snarkos_objects::Account;
use snarkos_storage::test_data::*;

use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

use snarkos_errors::objects::TransactionError;
use snarkos_models::objects::Transaction;
use snarkos_utilities::bytes::{FromBytes, ToBytes};
use std::io::{Read, Result as IoResult, Write};

pub static FIXTURE: Lazy<Fixture> = Lazy::new(|| setup(false));
pub static FIXTURE_VK: Lazy<Fixture> = Lazy::new(|| setup(true));

// helper for setting up e2e tests
pub struct Fixture {
    pub parameters: <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    pub test_accounts: [Account<Components>; 3],
    pub ledger_parameters: CommitmentMerkleParameters,
    pub genesis_attributes: GenesisAttributes,
    pub predicate: Predicate,
    pub rng: XorShiftRng,
}

impl Fixture {
    pub fn ledger(&self) -> MerkleTreeLedger {
        initialize_test_blockchain(
            self.ledger_parameters.clone(),
            self.genesis_attributes.genesis_cm,
            self.genesis_attributes.genesis_sn,
            self.genesis_attributes.genesis_memo,
            self.genesis_attributes.genesis_pred_vk_bytes.clone(),
            self.genesis_attributes.genesis_account_bytes.clone(),
        )
    }
}

fn setup(verify_only: bool) -> Fixture {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    // Generate or load parameters for the ledger, commitment schemes, and CRH
    let (ledger_parameters, parameters) = setup_or_load_parameters(verify_only, &mut rng);

    // Generate addresses
    let test_accounts = generate_test_accounts(&parameters, &mut rng);

    let genesis_attributes = ledger_genesis_setup(&parameters, &test_accounts[0], &mut rng);

    let predicate = Predicate::new(genesis_attributes.genesis_pred_vk_bytes.clone());

    Fixture {
        parameters,
        test_accounts,
        ledger_parameters,
        genesis_attributes,
        predicate,
        rng,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestTx;

impl Transaction for TestTx {
    type Commitment = [u8; 32];
    type Memorandum = [u8; 32];
    type SerialNumber = [u8; 32];
    type Stuff = [u8; 32];

    fn old_serial_numbers(&self) -> &[Self::SerialNumber] {
        &[[0u8; 32]]
    }

    fn new_commitments(&self) -> &[Self::Commitment] {
        &[[0u8; 32]]
    }

    fn memorandum(&self) -> &Self::Memorandum {
        &[0u8; 32]
    }

    fn stuff(&self) -> &Self::Stuff {
        &[0u8; 32]
    }

    fn transaction_id(&self) -> Result<[u8; 32], TransactionError> {
        Ok([0u8; 32])
    }

    fn size(&self) -> usize {
        0
    }

    fn value_balance(&self) -> i64 {
        0
    }
}

impl ToBytes for TestTx {
    #[inline]
    fn write<W: Write>(&self, mut _writer: W) -> IoResult<()> {
        Ok(())
    }
}

impl FromBytes for TestTx {
    #[inline]
    fn read<R: Read>(mut _reader: R) -> IoResult<Self> {
        Ok(Self)
    }
}
