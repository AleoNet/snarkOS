use crate::*;
use snarkos_algorithms::merkle_tree::*;
use snarkos_errors::dpc::LedgerError;
use snarkos_models::objects::{LedgerScheme, Transaction};
use snarkos_objects::Block;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use parking_lot::RwLock;
use rand::Rng;
use std::{fs, marker::PhantomData, path::PathBuf, sync::Arc};

impl<T: Transaction, P: MerkleParameters> LedgerScheme for LedgerStorage<T, P> {
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
        genesis_block: Self::Block,
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

        let leaves: Vec<[u8; 32]> = vec![];
        let empty_cm_merkle_tree = MerkleTree::<Self::MerkleParameters>::new(parameters.clone(), &leaves)?;

        let ledger_storage = Self {
            latest_block_height: RwLock::new(0),
            storage: Arc::new(storage),
            cm_merkle_tree: RwLock::new(empty_cm_merkle_tree),
            ledger_parameters: parameters,
            _transaction: PhantomData,
        };

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
        self.storage.exists(COL_SERIAL_NUMBER, &to_bytes![sn].unwrap())
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
