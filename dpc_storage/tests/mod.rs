use snarkos_dpc_storage::BlockStorage;

use snarkos_algorithms::{
    crh::{PedersenCompressedCRH, PedersenSize},
    merkle_tree::{MerkleParameters, MerkleTree},
};
use snarkos_curves::edwards_bls12::EdwardsProjective as EdwardsBls;
use snarkos_errors::objects::TransactionError;
use snarkos_models::algorithms::CRH;
use snarkos_objects::dpc::Transaction;

use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    storage::Storage,
};

use rand::Rng;
use std::{
    io::{Read, Result as IoResult, Write},
    path::PathBuf,
    sync::Arc,
};

pub const TEST_DB_PATH: &str = "../test_db";
pub const GENESIS_BLOCK: &str = "0000000000000000000000000000000000000000000000000000000000000000b3d9ad9de8e21b2b3a9ffb40bae6fefa852026e7fb2e279322cd7589a20ee35592ec145e00000000ffffffffff7f000030d901000101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff04080000000100e1f505000000001976a914ef5392fc02643be8b98f6aaca5c1ffaab238916a88ac";

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

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Size;
// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits
impl PedersenSize for Size {
    const NUM_WINDOWS: usize = 4;
    const WINDOW_SIZE: usize = 128;
}

type H = PedersenCompressedCRH<EdwardsBls, Size>;

#[derive(Clone)]
pub struct TestMerkleParams(H);

impl MerkleParameters for TestMerkleParams {
    type H = H;

    const HEIGHT: usize = 32;

    fn setup<R: Rng>(rng: &mut R) -> Self {
        Self(H::setup(rng))
    }

    fn crh(&self) -> &Self::H {
        &self.0
    }

    fn parameters(&self) -> &<<Self as MerkleParameters>::H as CRH>::Parameters {
        self.crh().parameters()
    }
}

impl Default for TestMerkleParams {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        Self(H::setup(&mut rng))
    }
}

impl Storage for TestMerkleParams {
    /// Store the SNARK proof to a file at the given path.
    fn store(&self, path: &PathBuf) -> IoResult<()> {
        self.0.store(path)
    }

    /// Load the SNARK proof from a file at the given path.
    fn load(path: &PathBuf) -> IoResult<Self> {
        Ok(Self(H::load(path)?))
    }
}

type Store = BlockStorage<TestTx, TestMerkleParams>;

pub fn initialize_test_blockchain() -> (Arc<Store>, PathBuf) {
    let mut path = std::env::current_dir().unwrap();
    path.push(random_storage_path());

    Store::destroy_storage(path.clone()).unwrap();

    let blockchain = Store::open_at_path(path.clone(), GENESIS_BLOCK.into()).unwrap();

    (blockchain, path)
}

pub fn random_storage_path() -> String {
    let ptr = Box::into_raw(Box::new(123));
    format!("{}{}", TEST_DB_PATH, ptr as usize)
}

pub fn kill_storage_async(path: PathBuf) {
    Store::destroy_storage(path).unwrap();
}

pub fn kill_storage_sync(storage: Arc<Store>, path: PathBuf) {
    drop(storage);
    Store::destroy_storage(path).unwrap();
}
