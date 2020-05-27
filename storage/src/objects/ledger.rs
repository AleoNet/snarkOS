use crate::*;
use snarkos_algorithms::merkle_tree::*;
use snarkos_errors::dpc::LedgerError;
use snarkos_models::objects::{Ledger, Transaction};
use snarkos_objects::{dpc::DPCTransactions, Block, BlockHeader, BlockHeaderHash, MerkleRootHash};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use parking_lot::RwLock;
use rand::Rng;
use std::{fs, marker::PhantomData, path::PathBuf, sync::Arc};

impl<T: Transaction, P: MerkleParameters> Ledger for LedgerStorage<T, P> {
    type Block = Block<Self::Transaction>;
    type Commitment = T::Commitment;
    type Memo = T::Memorandum;
    type MerkleParameters = P;
    type MerklePath = MerklePath<Self::MerkleParameters>;
    type MerkleTreeDigest = MerkleTreeDigest<Self::MerkleParameters>;
    type SerialNumber = T::SerialNumber;
    type Transaction = T;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self::MerkleParameters, LedgerError> {
        Ok(P::setup(rng))
    }

    fn new(
        path: &PathBuf,
        parameters: Self::MerkleParameters,
        genesis_cm: Self::Commitment,
        genesis_sn: Self::SerialNumber,
        genesis_memo: Self::Memo,
    ) -> Result<Self, LedgerError> {
        fs::create_dir_all(&path).map_err(|err| LedgerError::Message(err.to_string()))?;
        let storage = match Storage::open_cf(path, NUM_COLS) {
            Ok(storage) => storage,
            Err(err) => return Err(LedgerError::StorageError(err)),
        };

        if let Some(block_num) = storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())? {
            if bytes_to_u32(block_num) != 0 {
                return Err(LedgerError::ExistingDatabase);
            }
        }

        let cm_merkle_tree =
            MerkleTree::<Self::MerkleParameters>::new(parameters.clone(), &[genesis_cm.clone()]).unwrap();

        let header = BlockHeader {
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            merkle_root_hash: MerkleRootHash([0u8; 32]),
            time: 0,
            difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
            nonce: 0,
        };

        let genesis_block = Self::Block {
            header,
            transactions: DPCTransactions::new(),
        };

        let mut database_transaction = DatabaseTransaction::new();

        database_transaction.push(Op::Insert {
            col: COL_COMMITMENT,
            key: to_bytes![genesis_cm]?.to_vec(),
            value: (0 as u32).to_le_bytes().to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_CURR_CM_INDEX.as_bytes().to_vec(),
            value: (0 as u32).to_le_bytes().to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_CURR_SN_INDEX.as_bytes().to_vec(),
            value: (0 as u32).to_le_bytes().to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_CURR_MEMO_INDEX.as_bytes().to_vec(),
            value: (0 as u32).to_le_bytes().to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_DIGEST,
            key: to_bytes![cm_merkle_tree.root()]?.to_vec(),
            value: (0 as u32).to_le_bytes().to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_GENESIS_CM.as_bytes().to_vec(),
            value: to_bytes![genesis_cm]?.to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_GENESIS_SN.as_bytes().to_vec(),
            value: to_bytes![genesis_sn]?.to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_GENESIS_MEMO.as_bytes().to_vec(),
            value: to_bytes![genesis_memo]?.to_vec(),
        });

        let ledger_storage = Self {
            latest_block_height: RwLock::new(0),
            storage: Arc::new(storage),
            cm_merkle_tree: RwLock::new(cm_merkle_tree),
            ledger_parameters: parameters,
            _transaction: PhantomData,
        };

        ledger_storage.storage.write(database_transaction)?;
        ledger_storage.insert_block(&genesis_block)?;

        Ok(ledger_storage)
    }

    // Number of blocks including the genesis block
    fn len(&self) -> usize {
        self.get_latest_block_height() as usize + 1
    }

    fn parameters(&self) -> &Self::MerkleParameters {
        &self.ledger_parameters
    }

    fn push(&mut self, _transaction: Self::Transaction) -> Result<(), LedgerError> {
        unimplemented!()
    }

    fn digest(&self) -> Option<Self::MerkleTreeDigest> {
        let digest: Self::MerkleTreeDigest = FromBytes::read(&self.current_digest().unwrap()[..]).unwrap();

        Some(digest)
    }

    fn validate_digest(&self, digest: &Self::MerkleTreeDigest) -> bool {
        self.storage.exists(COL_DIGEST, &to_bytes![digest].unwrap())
    }

    fn contains_cm(&self, cm: &Self::Commitment) -> bool {
        self.storage.exists(COL_COMMITMENT, &to_bytes![cm].unwrap())
    }

    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool {
        self.storage.exists(COL_SERIAL_NUMBER, &to_bytes![sn].unwrap()) && sn != &self.genesis_sn().unwrap()
    }

    fn contains_memo(&self, memo: &Self::Memo) -> bool {
        self.storage.exists(COL_MEMO, &to_bytes![memo].unwrap())
    }

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<Self::MerklePath, LedgerError> {
        let cm_index = self.get_cm_index(&to_bytes![cm]?)?.ok_or(LedgerError::InvalidCmIndex)?;
        let result = self.cm_merkle_tree.read().generate_proof(cm_index, cm)?;

        Ok(result)
    }

    fn prove_sn(&self, _sn: &Self::SerialNumber) -> Result<Self::MerklePath, LedgerError> {
        Ok(MerklePath::default())
    }

    fn prove_memo(&self, _memo: &Self::Memo) -> Result<Self::MerklePath, LedgerError> {
        Ok(MerklePath::default())
    }

    fn verify_cm(
        _parameters: &Self::MerkleParameters,
        digest: &Self::MerkleTreeDigest,
        cm: &Self::Commitment,
        witness: &Self::MerklePath,
    ) -> bool {
        witness.verify(&digest, cm).unwrap()
    }

    fn verify_sn(
        _parameters: &Self::MerkleParameters,
        _digest: &Self::MerkleTreeDigest,
        _sn: &Self::SerialNumber,
        _witness: &Self::MerklePath,
    ) -> bool {
        true
    }

    fn verify_memo(
        _parameters: &Self::MerkleParameters,
        _digest: &Self::MerkleTreeDigest,
        _memo: &Self::Memo,
        _witness: &Self::MerklePath,
    ) -> bool {
        true
    }

    fn blocks(&self) -> &Vec<Self::Block> {
        unimplemented!()
    }
}
