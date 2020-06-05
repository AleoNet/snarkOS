use snarkos_algorithms::{
    crh::{PedersenCompressedCRH, PedersenSize},
    define_merkle_tree_parameters,
};
use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_errors::objects::TransactionError;
use snarkos_models::objects::{LedgerScheme, Transaction};
use snarkos_objects::Block;
use snarkos_storage::Ledger;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use rand::thread_rng;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
    sync::Arc,
};

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Size;

impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 128;
}

define_merkle_tree_parameters!(TestMerkleParams, PedersenCompressedCRH<EdwardsBls, Size>, 32);

pub type Store = Ledger<TestTx, TestMerkleParams>;

pub fn random_storage_path() -> String {
    let random_path: usize = thread_rng().gen();
    format!("./test_db-{}", random_path)
}

// Initialize a test blockchain given genesis attributes
pub fn initialize_test_blockchain<T: Transaction, P: MerkleParameters>(
    parameters: P,
    genesis_block: Block<T>,
) -> Ledger<T, P> {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    Ledger::<T, P>::destroy_storage(path.clone()).unwrap();

    let storage = Ledger::<T, P>::new(&path, parameters, genesis_block).unwrap();

    storage
}

// Open a test blockchain from stored genesis attributes
pub fn open_test_blockchain<T: Transaction, P: MerkleParameters>() -> (Arc<Ledger<T, P>>, PathBuf) {
    let mut path = std::env::temp_dir();
    path.push(random_storage_path());

    Ledger::<T, P>::destroy_storage(path.clone()).unwrap();

    let storage = Arc::new(Ledger::<T, P>::open_at_path(path.clone()).unwrap());

    (storage, path)
}

pub fn kill_storage<T: Transaction, P: MerkleParameters>(ledger: Ledger<T, P>) {
    let path = ledger.storage.db.path().to_owned();

    drop(ledger);
    Ledger::<T, P>::destroy_storage(path).unwrap();
}

pub fn kill_storage_async<T: Transaction, P: MerkleParameters>(path: PathBuf) {
    Ledger::<T, P>::destroy_storage(path).unwrap();
}

pub fn kill_storage_sync<T: Transaction, P: MerkleParameters>(ledger: Arc<Ledger<T, P>>) {
    let path = ledger.storage.db.path().to_owned();

    drop(ledger);
    Ledger::<T, P>::destroy_storage(path).unwrap();
}
