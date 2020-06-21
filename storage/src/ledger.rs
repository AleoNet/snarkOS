use crate::*;
use snarkos_algorithms::merkle_tree::MerkleTree;
use snarkos_errors::storage::StorageError;
use snarkos_models::{
    algorithms::MerkleParameters,
    genesis::Genesis,
    objects::{LedgerScheme, Transaction},
    parameters::Parameters,
};
use snarkos_objects::Block;
use snarkos_parameters::{GenesisBlock, LedgerMerkleTreeParameters};
use snarkos_utilities::bytes::FromBytes;

use parking_lot::RwLock;
use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Ledger<T: Transaction, P: MerkleParameters> {
    pub latest_block_height: RwLock<u32>,
    pub ledger_parameters: P,
    pub cm_merkle_tree: RwLock<MerkleTree<P>>,
    pub storage: Arc<Storage>,
    pub _transaction: PhantomData<T>,
}

impl<T: Transaction, P: MerkleParameters> Ledger<T, P> {
    /// Instantiates a new ledger storage.
    pub fn open() -> Result<Self, StorageError> {
        let mut path = std::env::current_dir()?;
        path.push("../db");

        Self::open_at_path(path)
    }

    /// Open the blockchain storage at a particular path.
    pub fn open_at_path<PATH: AsRef<Path>>(path: PATH) -> Result<Self, StorageError> {
        fs::create_dir_all(path.as_ref()).map_err(|err| StorageError::Message(err.to_string()))?;

        Self::load_ledger_state(path)
    }

    /// Returns true if there are no blocks in the ledger.
    pub fn is_empty(&self) -> bool {
        self.get_latest_block().is_err()
    }

    /// Get the latest block height of the chain.
    pub fn get_latest_block_height(&self) -> u32 {
        *self.latest_block_height.read()
    }

    /// Get the latest number of blocks in the chain.
    pub fn get_block_count(&self) -> u32 {
        *self.latest_block_height.read() + 1
    }

    /// Get the stored old connected peers.
    pub fn get_peer_book(&self) -> Result<Vec<u8>, StorageError> {
        Ok(self.get(COL_META, &KEY_PEER_BOOK.as_bytes().to_vec())?)
    }

    /// Store the connected peers.
    pub fn store_to_peer_book(&self, peers_serialized: Vec<u8>) -> Result<(), StorageError> {
        let op = Op::Insert {
            col: COL_META,
            key: KEY_PEER_BOOK.as_bytes().to_vec(),
            value: peers_serialized,
        };
        self.storage.write(DatabaseTransaction(vec![op]))
    }

    /// Destroy the storage given a path.
    pub fn destroy_storage(path: PathBuf) -> Result<(), StorageError> {
        Storage::destroy_storage(path)
    }

    /// Returns a `Ledger` with the latest state loaded from storage.
    fn load_ledger_state<PATH: AsRef<Path>>(path: PATH) -> Result<Self, StorageError> {
        let latest_block_number = {
            let storage = Storage::open_cf(path.as_ref(), NUM_COLS)?;
            storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())?
        };

        let crh = P::H::from(FromBytes::read(&LedgerMerkleTreeParameters::load_bytes()?[..])?);
        let ledger_parameters = P::from(crh);

        match latest_block_number {
            Some(val) => {
                let storage = Storage::open_cf(path.as_ref(), NUM_COLS)?;

                // Build commitment merkle tree

                let mut cm_and_indices = vec![];

                for (commitment_key, index_value) in storage.get_iter(COL_COMMITMENT)? {
                    let commitment: T::Commitment = FromBytes::read(&commitment_key[..])?;
                    let index = bytes_to_u32(index_value.to_vec()) as usize;

                    cm_and_indices.push((commitment, index));
                }

                cm_and_indices.sort_by(|&(_, i), &(_, j)| i.cmp(&j));
                let commitments = cm_and_indices.into_iter().map(|(cm, _)| cm).collect::<Vec<_>>();

                let merkle_tree = MerkleTree::new(ledger_parameters.clone(), &commitments)?;

                Ok(Self {
                    latest_block_height: RwLock::new(bytes_to_u32(val)),
                    storage: Arc::new(storage),
                    cm_merkle_tree: RwLock::new(merkle_tree),
                    ledger_parameters,
                    _transaction: PhantomData,
                })
            }
            None => {
                // Add genesis block to database

                let genesis_block: Block<T> = FromBytes::read(GenesisBlock::load_bytes().as_slice())?;

                let ledger_storage = Self::new(&path.as_ref().to_path_buf(), ledger_parameters, genesis_block)
                    .expect("Ledger could not be instantiated");

                Ok(ledger_storage)
            }
        }
    }

    /// Retrieve a value given a key.
    pub(crate) fn get(&self, col: u32, key: &Vec<u8>) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(col, key)? {
            Some(data) => Ok(data),
            None => Err(StorageError::MissingValue(hex::encode(key))),
        }
    }
}
