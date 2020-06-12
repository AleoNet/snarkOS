use crate::{
    dpc::{generate_test_accounts, setup_or_load_parameters},
    storage::*,
};
use snarkos_consensus::MerkleTreeLedger;
use snarkos_dpc::base_dpc::instantiated::*;
use snarkos_genesis::GenesisBlock;
use snarkos_models::{algorithms::CRH, dpc::DPCScheme, genesis::Genesis};
use snarkos_objects::{Account, Block};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use once_cell::sync::Lazy;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

use snarkos_errors::objects::TransactionError;
use snarkos_models::objects::Transaction;
use std::io::{Read, Result as IoResult, Write};

pub static FIXTURE: Lazy<Fixture> = Lazy::new(|| setup(false));
pub static FIXTURE_VK: Lazy<Fixture> = Lazy::new(|| setup(true));

// helper for setting up e2e tests
pub struct Fixture {
    pub parameters: <InstantiatedDPC as DPCScheme<MerkleTreeLedger>>::Parameters,
    pub test_accounts: [Account<Components>; 3],
    pub ledger_parameters: CommitmentMerkleParameters,
    pub genesis_block: Block<Tx>,
    pub predicate: Predicate,
    pub rng: XorShiftRng,
}

impl Fixture {
    pub fn ledger(&self) -> MerkleTreeLedger {
        initialize_test_blockchain(self.ledger_parameters.clone(), self.genesis_block.clone())
    }
}

fn setup(verify_only: bool) -> Fixture {
    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    // Generate or load parameters for the ledger, commitment schemes, and CRH
    let (ledger_parameters, parameters) = setup_or_load_parameters(verify_only, &mut rng);

    // Generate addresses
    let test_accounts = generate_test_accounts(&parameters, &mut rng);

    let genesis_block: Block<Tx> = FromBytes::read(GenesisBlock::load_bytes().as_slice()).unwrap();

    let predicate_vk_hash = to_bytes![
        PredicateVerificationKeyHash::hash(
            &parameters.circuit_parameters.predicate_verification_key_hash,
            &to_bytes![parameters.predicate_snark_parameters().verification_key].unwrap()
        )
        .unwrap()
    ]
    .unwrap();

    let predicate = Predicate::new(predicate_vk_hash);

    Fixture {
        parameters,
        test_accounts,
        ledger_parameters,
        genesis_block,
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

    fn old_serial_numbers(&self) -> &[Self::SerialNumber] {
        &[[0u8; 32]]
    }

    fn new_commitments(&self) -> &[Self::Commitment] {
        &[[0u8; 32]]
    }

    fn memorandum(&self) -> &Self::Memorandum {
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
