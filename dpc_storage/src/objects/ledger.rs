use crate::*;

use snarkos_algorithms::merkle_tree::*;
use snarkos_errors::dpc::LedgerError;
use snarkos_objects::{
    dpc::{Block, Transaction},
    ledger::Ledger,
    BlockHeaderHash,
};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

use rand::Rng;
use std::{collections::HashSet, hash::Hash};

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
        parameters: Self::Parameters,
        genesis_cm: Self::Commitment,
        genesis_sn: Self::SerialNumber,
        genesis_memo: Self::Memo,
    ) -> Self {
        //        let cm_merkle_tree = MerkleTree::<Self::Parameters>::new(&parameters, &[genesis_cm.clone()]).unwrap();
        //
        //        let mut cur_cm_index = 0;
        //        let mut comm_to_index = HashMap::new();
        //        comm_to_index.insert(genesis_cm.clone(), cur_cm_index);
        //        cur_cm_index += 1;
        //
        //        let root = cm_merkle_tree.root();
        //        let mut past_digests = HashSet::new();
        //        past_digests.insert(root.clone());
        //
        //        let time = SystemTime::now()
        //            .duration_since(UNIX_EPOCH)
        //            .expect("Time went backwards")
        //            .as_secs() as i64;
        //
        //        let header = BlockHeader {
        //            previous_block_hash: BlockHeaderHash([0u8; 32]),
        //            merkle_root_hash: MerkleRootHash([0u8; 32]),
        //            time,
        //            difficulty_target: 0x07FF_FFFF_FFFF_FFFF_u64,
        //            nonce: 0,
        //        };
        //
        //        let genesis_block = Block::<T> {
        //            header,
        //            transactions: DPCTransactions::new(),
        //        };
        //
        //        Self {
        //            crh_params: Rc::new(parameters),
        //            blocks: vec![genesis_block],
        //            cm_merkle_tree,
        //            cur_cm_index,
        //            cur_sn_index: 0,
        //            cur_memo_index: 0,
        //
        //            comm_to_index,
        //            sn_to_index: HashMap::new(),
        //            memo_to_index: HashMap::new(),
        //            current_digest: Some(root),
        //            past_digests,
        //            genesis_cm,
        //            genesis_sn,
        //            genesis_memo,
        //        }
        unimplemented!()
    }

    fn len(&self) -> usize {
        unimplemented!()
    }

    fn parameters(&self) -> &Self::Parameters {
        unimplemented!()
    }

    fn push(&mut self, transaction: Self::Transaction) -> Result<(), LedgerError> {
        unimplemented!()
    }

    fn digest(&self) -> Option<MerkleTreeDigest<Self::Parameters>> {
        unimplemented!()
    }

    fn validate_digest(&self, digest: &MerkleTreeDigest<Self::Parameters>) -> bool {
        unimplemented!()
    }

    fn contains_cm(&self, cm: &Self::Commitment) -> bool {
        unimplemented!()
    }

    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool {
        unimplemented!()
    }

    fn contains_memo(&self, memo: &Self::Memo) -> bool {
        unimplemented!()
    }

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<MerklePath<Self::Parameters>, LedgerError> {
        //        let cm_index = self.comm_to_index.get(cm).ok_or(LedgerError::InvalidCmIndex)?;
        //
        //        let result = self.cm_merkle_tree.generate_proof(*cm_index, cm)?;
        //
        //        Ok(result)
        unimplemented!()
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
