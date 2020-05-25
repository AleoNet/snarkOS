use crate::*;
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerkleTree};
use snarkos_errors::storage::StorageError;
use snarkos_models::objects::{Ledger, Transaction};
use snarkos_objects::{dpc::DPCTransactions, BlockHeader, BlockHeaderHash};
use snarkos_parameters::LedgerMerkleTreeParameters;
use snarkos_utilities::bytes::FromBytes;

use parking_lot::RwLock;
use std::{
    fs,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct LedgerStorage<T: Transaction, P: MerkleParameters> {
    pub latest_block_height: RwLock<u32>,
    pub ledger_parameters: P,
    pub cm_merkle_tree: RwLock<MerkleTree<P>>,
    pub storage: Arc<Storage>,
    pub _transaction: PhantomData<T>,
}

impl<T: Transaction, P: MerkleParameters> LedgerStorage<T, P> {
    /// Create a new blockchain storage.
    pub fn open() -> Result<Self, StorageError> {
        let mut path = std::env::current_dir()?;
        path.push("../db");

        Self::open_at_path(path)
    }

    /// Open the blockchain storage at a particular path.
    pub fn open_at_path<PATH: AsRef<Path>>(path: PATH) -> Result<Self, StorageError> {
        fs::create_dir_all(path.as_ref()).map_err(|err| StorageError::Message(err.to_string()))?;

        Self::get_latest_state(path)
    }

    /// Get the latest state of the storage.
    pub fn get_latest_state<PATH: AsRef<Path>>(path: PATH) -> Result<Self, StorageError> {
        let latest_block_number = {
            let storage = Storage::open_cf(path.as_ref(), NUM_COLS)?;
            storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())?
        };

        let crh = P::H::from(FromBytes::read(&LedgerMerkleTreeParameters::load_bytes()[..])?);
        let ledger_parameters = P::from(crh);

        match latest_block_number {
            Some(val) => {
                let storage = Storage::open_cf(path.as_ref(), NUM_COLS)?;

                let mut cm_and_indices = vec![];

                for (commitment_key, index_value) in storage.get_iter(COL_COMMITMENT)? {
                    let commitment: T::Commitment = FromBytes::read(&commitment_key[..])?;
                    let index = bytes_to_u32(index_value.to_vec()) as usize;

                    cm_and_indices.push((commitment, index));
                }

                cm_and_indices.sort_by(|&(_, i), &(_, j)| i.cmp(&j));
                let commitments = cm_and_indices.into_iter().map(|(cm, _)| cm).collect::<Vec<_>>();

                let genesis_cm: T::Commitment = match storage.get(COL_META, KEY_GENESIS_CM.as_bytes())? {
                    Some(cm_bytes) => FromBytes::read(&cm_bytes[..])?,
                    None => return Err(StorageError::MissingGenesisCm),
                };

                assert!(commitments[0] == genesis_cm);

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

                let ledger_storage = Self::new(
                    &path.as_ref().to_path_buf(),
                    ledger_parameters,
                    FromBytes::read(&GENESIS_RECORD_COMMITMENT[..])?,
                    FromBytes::read(&GENESIS_SERIAL_NUMBER[..])?,
                    FromBytes::read(&GENESIS_MEMO[..])?,
                    GENESIS_PRED_VK_BYTES.to_vec(),
                    GENESIS_ACCOUNT.to_vec(),
                )
                .unwrap(); // TODO handle this unwrap. merge storage and ledger error

                Ok(ledger_storage)
            }
        }
    }

    /// Get the latest block height of the chain.
    pub fn get_latest_block_height(&self) -> u32 {
        *self.latest_block_height.read()
    }

    /// Get the latest number of blocks in the chain.
    pub fn get_block_count(&self) -> u32 {
        *self.latest_block_height.read() + 1
    }

    /// Destroy the storage given a path.
    pub fn destroy_storage(path: PathBuf) -> Result<(), StorageError> {
        Storage::destroy_storage(path)
    }

    /// Retrieve a value given a key.
    pub(crate) fn get(&self, col: u32, key: &Vec<u8>) -> Result<Vec<u8>, StorageError> {
        match self.storage.get(col, key)? {
            Some(data) => Ok(data),
            None => Err(StorageError::MissingValue(hex::encode(key))),
        }
    }

    // KEY VALUE GETTERS ===========================================================================

    /// Get the stored memory pool transactions.
    pub fn get_memory_pool(&self) -> Result<Vec<u8>, StorageError> {
        Ok(self.get(COL_META, &KEY_MEMORY_POOL.as_bytes().to_vec())?)
    }

    /// Store the memory pool transactions.
    pub fn store_to_memory_pool(&self, transactions_serialized: Vec<u8>) -> Result<(), StorageError> {
        let op = Op::Insert {
            col: COL_META,
            key: KEY_MEMORY_POOL.as_bytes().to_vec(),
            value: transactions_serialized,
        };
        self.storage.write(DatabaseTransaction(vec![op]))
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

    /// Get a block header given the block hash.
    pub fn get_block_header(&self, block_hash: &BlockHeaderHash) -> Result<BlockHeader, StorageError> {
        match self.storage.get(COL_BLOCK_HEADER, &block_hash.0)? {
            Some(block_header_bytes) => Ok(BlockHeader::read(&block_header_bytes[..])?),
            None => Err(StorageError::MissingBlockHeader(block_hash.to_string())),
        }
    }

    /// Get the block hash given a block number.
    pub fn get_block_hash(&self, block_num: u32) -> Result<BlockHeaderHash, StorageError> {
        match self.storage.get(COL_BLOCK_LOCATOR, &block_num.to_le_bytes())? {
            Some(block_header_hash) => Ok(BlockHeaderHash::new(block_header_hash)),
            None => Err(StorageError::MissingBlockHash(block_num)),
        }
    }

    /// Get the block num given a block hash.
    pub fn get_block_num(&self, block_hash: &BlockHeaderHash) -> Result<u32, StorageError> {
        match self.storage.get(COL_BLOCK_LOCATOR, &block_hash.0)? {
            Some(block_num_bytes) => Ok(bytes_to_u32(block_num_bytes)),
            None => Err(StorageError::MissingBlockNumber(block_hash.to_string())),
        }
    }

    /// Get the list of transaction ids given a block hash.
    pub fn get_block_transactions(&self, block_hash: &BlockHeaderHash) -> Result<DPCTransactions<T>, StorageError> {
        match self.storage.get(COL_BLOCK_TRANSACTIONS, &block_hash.0)? {
            Some(encoded_block_transactions) => Ok(DPCTransactions::read(&encoded_block_transactions[..])?),
            None => Err(StorageError::MissingBlockTransactions(block_hash.to_string())),
        }
    }

    /// Find the potential child block given a parent block header.
    pub fn get_child_hash(&self, _parent_header: &BlockHeaderHash) -> Result<BlockHeaderHash, StorageError> {
        unimplemented!()
    }

    /// Get a transaction given the transaction id.
    pub fn get_transaction(&self, transaction_id: &Vec<u8>) -> Result<Option<T>, StorageError> {
        match self.storage.get(COL_TRANSACTION_LOCATION, &transaction_id)? {
            Some(transaction_locator) => {
                let transaction_location = TransactionLocation::read(&transaction_locator[..])?;
                let block_transactions =
                    self.get_block_transactions(&BlockHeaderHash(transaction_location.block_hash))?;
                Ok(Some(block_transactions.0[transaction_location.index as usize].clone()))
            }
            None => Ok(None),
        }
    }
}
