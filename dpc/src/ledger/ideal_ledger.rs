use crate::Error;
use rand::Rng;
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
    rc::Rc,
};

use crate::{dpc::Transaction, ledger::*};
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTree, MerkleTreeDigest};
use snarkos_models::algorithms::CRH;
use snarkos_utilities::bytes::ToBytes;

pub struct IdealLedger<T: Transaction, P: MerkleParameters>
where
    T::Commitment: ToBytes,
{
    crh_params:     Rc<<P::H as CRH>::Parameters>,
    transactions:   Vec<T>,
    cm_merkle_tree: MerkleTree<P>,
    cur_cm_index:   usize,
    cur_sn_index:   usize,
    cur_memo_index: usize,
    comm_to_index:  HashMap<T::Commitment, usize>,
    sn_to_index:    HashMap<T::SerialNumber, usize>,
    memo_to_index:  HashMap<T::Memorandum, usize>,
    current_digest: Option<MerkleTreeDigest<P>>,
    past_digests:   HashSet<MerkleTreeDigest<P>>,
    genesis_cm:     T::Commitment,
    genesis_sn:     T::SerialNumber,
    genesis_memo:   T::Memorandum,
}

impl<T: Transaction, P: MerkleParameters> Ledger for IdealLedger<T, P>
where
    T: Eq,
    T::Commitment: ToBytes + Clone,
    T::SerialNumber: ToBytes + Clone,
    T::Memorandum: Hash + Clone,
{
    type Parameters = P;

    type Commitment = T::Commitment;
    type SerialNumber = T::SerialNumber;
    type Memo = T::Memorandum;
    type Transaction = T;

    fn setup<R: Rng>(rng: &mut R) -> Result<MerkleTreeParams<Self::Parameters>, Error> {
        Ok(P::H::setup(rng))
    }

    fn new(
        parameters: <P::H as CRH>::Parameters,
        genesis_cm: Self::Commitment,
        genesis_sn: Self::SerialNumber,
        genesis_memo: Self::Memo,
    ) -> Self {
        let params = Rc::new(parameters);
        let cm_merkle_tree = MerkleTree::<P>::new(&[genesis_cm.clone()]).unwrap();

        let mut cur_cm_index = 0;
        let mut comm_to_index = HashMap::new();
        comm_to_index.insert(genesis_cm.clone(), cur_cm_index);
        cur_cm_index += 1;

        let root = cm_merkle_tree.root();
        let mut past_digests = HashSet::new();
        past_digests.insert(root.clone());

        IdealLedger {
            crh_params: params,
            transactions: Vec::new(),
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
        }
    }

    fn len(&self) -> usize {
        self.transactions.len()
    }

    fn parameters(&self) -> &MerkleTreeParams<Self::Parameters> {
        &self.crh_params
    }

    fn push(&mut self, transaction: Self::Transaction) -> Result<(), Error> {
        let push_time = start_timer!(|| "IdealLedger::PushTx");

        let mut cur_sn_index = self.cur_sn_index;
        for sn in transaction.old_serial_numbers() {
            if sn != &self.genesis_sn {
                if self.sn_to_index.contains_key(sn) {
                    Err(LedgerError::DuplicateSn)?;
                }
                self.sn_to_index.insert(sn.clone(), cur_sn_index);
                cur_sn_index += 1;
            }
        }
        self.cur_sn_index = cur_sn_index;

        let mut cur_cm_index = self.cur_cm_index;
        for cm in transaction.new_commitments() {
            if cm == &self.genesis_cm || self.comm_to_index.contains_key(cm) {
                Err(LedgerError::InvalidCm)?;
            }
            self.comm_to_index.insert(cm.clone(), cur_cm_index);
            cur_cm_index += 1;
        }
        self.cur_cm_index = cur_cm_index;

        if transaction.memorandum() != &self.genesis_memo {
            if self.memo_to_index.contains_key(transaction.memorandum()) {
                Err(LedgerError::DuplicateMemo)?;
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
        self.cm_merkle_tree = MerkleTree::new(&commitments)?;

        let new_digest = self.cm_merkle_tree.root();
        self.past_digests.insert(new_digest.clone());
        self.current_digest = Some(new_digest);

        self.transactions.push(transaction);

        end_timer!(push_time);
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

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<MerklePath<Self::Parameters>, Error> {
        let witness_time = start_timer!(|| "Generate membership witness");

        let cm_index = self
            .comm_to_index
            .get(cm)
            .ok_or(LedgerError::InvalidCmIndex)?;

        let result = self.cm_merkle_tree.generate_proof(*cm_index, cm)?;

        end_timer!(witness_time);
        Ok(result)
    }

    fn prove_sn(&self, _sn: &Self::SerialNumber) -> Result<MerklePath<Self::Parameters>, Error> {
        Ok(MerklePath::default())
    }

    fn prove_memo(&self, _memo: &Self::Memo) -> Result<MerklePath<Self::Parameters>, Error> {
        Ok(MerklePath::default())
    }


    fn verify_cm(
        parameters: &MerkleTreeParams<Self::Parameters>,
        digest: &MerkleTreeDigest<Self::Parameters>,
        cm: &Self::Commitment,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool {
        witness.verify(&digest, cm).unwrap()
    }

    fn verify_sn(
        _parameters: &MerkleTreeParams<Self::Parameters>,
        _digest: &MerkleTreeDigest<Self::Parameters>,
        _sn: &Self::SerialNumber,
        _witness: &MerklePath<Self::Parameters>,
    ) -> bool {
        true
    }

    fn verify_memo(
        _parameters: &MerkleTreeParams<Self::Parameters>,
        _digest: &MerkleTreeDigest<Self::Parameters>,
        _memo: &Self::Memo,
        _witness: &MerklePath<Self::Parameters>,
    ) -> bool {
        true
    }
}

#[derive(Debug)]
pub enum LedgerError {
    DuplicateSn,
    DuplicateMemo,
    InvalidCm,
    InvalidCmIndex,
}

impl std::fmt::Display for LedgerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            LedgerError::DuplicateSn => "duplicate sn pushed to ledger",
            LedgerError::DuplicateMemo => "duplicate memo pushed to ledger",
            LedgerError::InvalidCm => "invalid cm pushed to ledger",
            LedgerError::InvalidCmIndex => "invalid cm index during proving",

        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for LedgerError {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}


