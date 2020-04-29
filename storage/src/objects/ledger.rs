use crate::*;

use snarkos_algorithms::merkle_tree::*;
use snarkos_errors::dpc::LedgerError;
use snarkos_objects::{
    dpc::{Block, DPCTransactions, Transaction},
    ledger::Ledger,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use parking_lot::RwLock;
use rand::Rng;
use std::{fs, marker::PhantomData, path::PathBuf, sync::Arc};

impl<T: Transaction, P: MerkleParameters> Ledger for BlockStorage<T, P> {
    type Commitment = T::Commitment;
    type Memo = T::Memorandum;
    type Parameters = P;
    type SerialNumber = T::SerialNumber;
    type Transaction = T;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self::Parameters, LedgerError> {
        Ok(P::setup(rng))
    }

    fn new(
        path: &PathBuf,
        parameters: Self::Parameters,
        genesis_cm: Self::Commitment,
        genesis_sn: Self::SerialNumber,
        genesis_memo: Self::Memo,
        genesis_predicate_vk_bytes: Vec<u8>,
        genesis_address_pair_bytes: Vec<u8>,
    ) -> Result<Self, LedgerError> {
        fs::create_dir_all(&path).map_err(|err| LedgerError::Message(err.to_string()))?;
        let storage = match Storage::open_cf(path, NUM_COLS) {
            Ok(storage) => storage,
            Err(err) => return Err(LedgerError::StorageError(err)),
        };

        if let Some(block_num) = storage.get(COL_META, KEY_BEST_BLOCK_NUMBER.as_bytes())? {
            if bytes_to_u32(block_num) != 0 {
                return Err(LedgerError::Message("Existing database".into()));
            }
        }

        let cm_merkle_tree = MerkleTree::<Self::Parameters>::new(&parameters, &[genesis_cm.clone()]).unwrap();

        let header = BlockHeader {
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            merkle_root_hash: MerkleRootHash([0u8; 32]),
            time: 0,
            difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
            nonce: 0,
        };

        let genesis_block = Block::<T> {
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
            value: (1 as u32).to_le_bytes().to_vec(),
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

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_GENESIS_PRED_VK.as_bytes().to_vec(),
            value: genesis_predicate_vk_bytes,
        });

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_GENESIS_ADDRESS_PAIR.as_bytes().to_vec(),
            value: genesis_address_pair_bytes,
        });

        let block_storage = Self {
            latest_block_height: RwLock::new(0),
            storage: Arc::new(storage),
            cm_merkle_tree: RwLock::new(cm_merkle_tree),
            ledger_parameters: parameters,
            _transaction: PhantomData,
        };

        block_storage.storage.write(database_transaction)?;
        block_storage.insert_block(&genesis_block)?;

        Ok(block_storage)
    }

    // Number of blocks including the genesis block
    fn len(&self) -> usize {
        self.get_latest_block_height() as usize + 1
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.ledger_parameters
    }

    fn push(&mut self, _transaction: Self::Transaction) -> Result<(), LedgerError> {
        unimplemented!()
    }

    fn digest(&self) -> Option<MerkleTreeDigest<Self::Parameters>> {
        let digest: MerkleTreeDigest<Self::Parameters> = FromBytes::read(&self.current_digest().unwrap()[..]).unwrap();

        Some(digest)
    }

    fn validate_digest(&self, digest: &MerkleTreeDigest<Self::Parameters>) -> bool {
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

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<MerklePath<Self::Parameters>, LedgerError> {
        let cm_index = self.get_cm_index(&to_bytes![cm]?)?.ok_or(LedgerError::InvalidCmIndex)?;
        let result = self.cm_merkle_tree.read().generate_proof(cm_index, cm)?;

        Ok(result)
    }

    fn prove_sn(&self, _sn: &Self::SerialNumber) -> Result<MerklePath<Self::Parameters>, LedgerError> {
        Ok(MerklePath::default())
    }

    fn prove_memo(&self, _memo: &Self::Memo) -> Result<MerklePath<Self::Parameters>, LedgerError> {
        Ok(MerklePath::default())
    }

    fn verify_cm(
        _parameters: &Self::Parameters,
        digest: &MerkleTreeDigest<Self::Parameters>,
        cm: &Self::Commitment,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool {
        witness.verify(&digest, cm).unwrap()
    }

    fn verify_sn(
        _parameters: &Self::Parameters,
        _digest: &MerkleTreeDigest<Self::Parameters>,
        _sn: &Self::SerialNumber,
        _witness: &MerklePath<Self::Parameters>,
    ) -> bool {
        true
    }

    fn verify_memo(
        _parameters: &Self::Parameters,
        _digest: &MerkleTreeDigest<Self::Parameters>,
        _memo: &Self::Memo,
        _witness: &MerklePath<Self::Parameters>,
    ) -> bool {
        true
    }

    fn blocks(&self) -> &Vec<Block<Self::Transaction>> {
        unimplemented!()
    }
}
