use crate::{
    dpc::{Block, DPCTransactions, Transaction},
    ledger::*,
    BlockHeader,
    BlockHeaderHash,
    MerkleRootHash,
};

use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTree, MerkleTreeDigest};
use snarkos_errors::dpc::LedgerError;

use rand::Rng;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    path::PathBuf,
    rc::Rc,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct BasicLedger<T: Transaction, P: MerkleParameters> {
    crh_params: Rc<P>,
    blocks: Vec<Block<T>>,
    cm_merkle_tree: MerkleTree<P>,
    cur_cm_index: usize,
    cur_sn_index: usize,
    cur_memo_index: usize,
    comm_to_index: HashMap<T::Commitment, usize>,
    sn_to_index: HashMap<T::SerialNumber, usize>,
    memo_to_index: HashMap<T::Memorandum, usize>,
    current_digest: Option<MerkleTreeDigest<P>>,
    past_digests: HashSet<MerkleTreeDigest<P>>,
    genesis_cm: T::Commitment,
    genesis_sn: T::SerialNumber,
    genesis_memo: T::Memorandum,
}

/// Check if an iterator has duplicate elements
pub fn has_duplicates<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    !iter.into_iter().all(move |x| uniq.insert(x))
}

impl<T: Transaction, P: MerkleParameters> Ledger for BasicLedger<T, P> {
    type Commitment = T::Commitment;
    type Memo = T::Memorandum;
    type Parameters = P;
    type SerialNumber = T::SerialNumber;
    type Transaction = T;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self::Parameters, LedgerError> {
        Ok(P::setup(rng))
    }

    fn new(
        _path: &PathBuf,
        parameters: Self::Parameters,
        genesis_cm: Self::Commitment,
        genesis_sn: Self::SerialNumber,
        genesis_memo: Self::Memo,
        _genesis_predicate_vk_bytes: Vec<u8>,
        _genesis_address_pair_bytes: Vec<u8>,
    ) -> Result<Self, LedgerError> {
        let cm_merkle_tree = MerkleTree::<Self::Parameters>::new(&parameters, &[genesis_cm.clone()])?;

        let mut cur_cm_index = 0;
        let mut comm_to_index = HashMap::new();
        comm_to_index.insert(genesis_cm.clone(), cur_cm_index);
        cur_cm_index += 1;

        let root = cm_merkle_tree.root();
        let mut past_digests = HashSet::new();
        past_digests.insert(root.clone());

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs() as i64;

        let header = BlockHeader {
            previous_block_hash: BlockHeaderHash([0u8; 32]),
            merkle_root_hash: MerkleRootHash([0u8; 32]),
            time,
            difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
            nonce: 0,
        };

        let genesis_block = Block::<T> {
            header,
            transactions: DPCTransactions::new(),
        };

        Ok(Self {
            crh_params: Rc::new(parameters),
            blocks: vec![genesis_block],
            cm_merkle_tree,
            cur_cm_index,
            cur_sn_index: 0,
            cur_memo_index: 0,

            comm_to_index,
            sn_to_index: HashMap::new(),
            memo_to_index: HashMap::new(),
            current_digest: Some(root),
            past_digests,
            genesis_cm,
            genesis_sn,
            genesis_memo,
        })
    }

    fn len(&self) -> usize {
        self.blocks.len()
    }

    fn parameters(&self) -> &Self::Parameters {
        &self.crh_params
    }

    fn push(&mut self, transaction: Self::Transaction) -> Result<(), LedgerError> {
        let mut cur_sn_index = self.cur_sn_index;
        for sn in transaction.old_serial_numbers() {
            if sn != &self.genesis_sn {
                if self.sn_to_index.contains_key(sn) {
                    return Err(LedgerError::DuplicateSn);
                }
                self.sn_to_index.insert(sn.clone(), cur_sn_index);
                cur_sn_index += 1;
            }
        }
        self.cur_sn_index = cur_sn_index;

        let mut cur_cm_index = self.cur_cm_index;
        for cm in transaction.new_commitments() {
            if cm == &self.genesis_cm || self.comm_to_index.contains_key(cm) {
                return Err(LedgerError::InvalidCm);
            }
            self.comm_to_index.insert(cm.clone(), cur_cm_index);
            cur_cm_index += 1;
        }
        self.cur_cm_index = cur_cm_index;

        if transaction.memorandum() != &self.genesis_memo {
            if self.memo_to_index.contains_key(transaction.memorandum()) {
                return Err(LedgerError::DuplicateMemo);
            } else {
                self.memo_to_index
                    .insert(transaction.memorandum().clone(), self.cur_memo_index);
                self.cur_memo_index += 1;
            }
        }

        // Rebuild the tree.
        let mut cm_and_indices = self.comm_to_index.iter().collect::<Vec<_>>();
        cm_and_indices.sort_by(|&(_, i), &(_, j)| i.cmp(j));
        let commitments = cm_and_indices
            .into_iter()
            .map(|(cm, _)| cm)
            .cloned()
            .collect::<Vec<_>>();
        assert!(commitments[0] == self.genesis_cm);
        self.cm_merkle_tree = MerkleTree::new(self.parameters(), &commitments)?;

        let new_digest = self.cm_merkle_tree.root();
        self.past_digests.insert(new_digest.clone());
        self.current_digest = Some(new_digest);

        //        self.transactions.push(transaction);

        Ok(())
    }

    fn digest(&self) -> Option<MerkleTreeDigest<Self::Parameters>> {
        self.current_digest.clone()
    }

    fn validate_digest(&self, digest: &MerkleTreeDigest<Self::Parameters>) -> bool {
        self.past_digests.contains(digest)
    }

    fn contains_cm(&self, cm: &Self::Commitment) -> bool {
        self.comm_to_index.contains_key(cm)
    }

    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool {
        self.sn_to_index.contains_key(sn) && sn != &self.genesis_sn
    }

    fn contains_memo(&self, memo: &Self::Memo) -> bool {
        self.memo_to_index.contains_key(memo)
    }

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<MerklePath<Self::Parameters>, LedgerError> {
        let cm_index = self.comm_to_index.get(cm).ok_or(LedgerError::InvalidCmIndex)?;

        let result = self.cm_merkle_tree.generate_proof(*cm_index, cm)?;

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
        &self.blocks
    }
}

impl<T: Transaction, P: MerkleParameters> BasicLedger<T, P> {
    pub fn process_transaction(&mut self, transaction: &T) -> Result<(), LedgerError> {
        let mut cur_sn_index = self.cur_sn_index;
        for sn in transaction.old_serial_numbers() {
            if sn != &self.genesis_sn {
                if self.sn_to_index.contains_key(sn) {
                    return Err(LedgerError::DuplicateSn);
                }
                self.sn_to_index.insert(sn.clone(), cur_sn_index);
                cur_sn_index += 1;
            }
        }
        self.cur_sn_index = cur_sn_index;

        let mut cur_cm_index = self.cur_cm_index;
        for cm in transaction.new_commitments() {
            if cm == &self.genesis_cm || self.comm_to_index.contains_key(cm) {
                return Err(LedgerError::InvalidCm);
            }
            self.comm_to_index.insert(cm.clone(), cur_cm_index);
            cur_cm_index += 1;
        }
        self.cur_cm_index = cur_cm_index;

        if transaction.memorandum() != &self.genesis_memo {
            if self.memo_to_index.contains_key(transaction.memorandum()) {
                return Err(LedgerError::DuplicateMemo);
            } else {
                self.memo_to_index
                    .insert(transaction.memorandum().clone(), self.cur_memo_index);
                self.cur_memo_index += 1;
            }
        }

        Ok(())
    }

    pub fn push_block(&mut self, block: Block<T>) -> Result<(), LedgerError> {
        let mut transaction_serial_numbers = vec![];
        let mut transaction_commitments = vec![];
        let mut transaction_memos = vec![];

        for transaction in &block.transactions.0 {
            transaction_serial_numbers.push(transaction.transaction_id()?);
            transaction_commitments.push(transaction.new_commitments());
            transaction_memos.push(transaction.memorandum());
        }

        // Check if the transactions in the block have duplicate serial numbers
        if has_duplicates(transaction_serial_numbers) {
            return Err(LedgerError::DuplicateSn);
        }

        // Check if the transactions in the block have duplicate commitments
        if has_duplicates(transaction_commitments) {
            return Err(LedgerError::InvalidCm);
        }

        // Check if the transactions in the block have duplicate memos
        if has_duplicates(transaction_memos) {
            return Err(LedgerError::DuplicateMemo);
        }

        // Process the transactions

        for transaction in &block.transactions.0 {
            self.process_transaction(transaction)?;
        }

        // Rebuild the tree.
        let mut cm_and_indices = self.comm_to_index.iter().collect::<Vec<_>>();
        cm_and_indices.sort_by(|&(_, i), &(_, j)| i.cmp(j));
        let commitments = cm_and_indices
            .into_iter()
            .map(|(cm, _)| cm)
            .cloned()
            .collect::<Vec<_>>();
        assert!(commitments[0] == self.genesis_cm);
        self.cm_merkle_tree = MerkleTree::new(self.parameters(), &commitments)?;

        let new_digest = self.cm_merkle_tree.root();
        self.past_digests.insert(new_digest.clone());
        self.current_digest = Some(new_digest);

        self.blocks.push(block);

        Ok(())
    }
}
