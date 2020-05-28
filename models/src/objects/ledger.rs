use crate::objects::{BlockScheme, Transaction};
use snarkos_errors::dpc::LedgerError;

use rand::Rng;
use std::path::PathBuf;

pub trait Ledger: Sized {
    type Block: BlockScheme;
    type Commitment;
    type Memo;
    type MerkleParameters;
    type MerklePath;
    type MerkleTreeDigest;
    type SerialNumber;
    type Transaction: Transaction;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self::MerkleParameters, LedgerError>;

    /// Creates an empty ledger
    fn new(path: &PathBuf, parameters: Self::MerkleParameters, genesis_block: Self::Block)
    -> Result<Self, LedgerError>;

    /// Return the current number of transactions on the ledger.
    fn len(&self) -> usize;

    /// Return the parameters used to construct the ledger data structure.
    fn parameters(&self) -> &Self::MerkleParameters;

    /// Append a (valid) transaction tx to the ledger.
    fn push(&mut self, transaction: Self::Transaction) -> Result<(), LedgerError>;

    /// Return a short digest of the current state of the transaction set data
    /// structure.
    fn digest(&self) -> Option<Self::MerkleTreeDigest>;

    /// Check that st_{ts} is a valid digest for some (past) ledger state.
    fn validate_digest(&self, digest: &Self::MerkleTreeDigest) -> bool;

    fn contains_cm(&self, cm: &Self::Commitment) -> bool;
    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool;
    fn contains_memo(&self, memo: &Self::Memo) -> bool;

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<Self::MerklePath, LedgerError>;
    fn prove_sn(&self, sn: &Self::SerialNumber) -> Result<Self::MerklePath, LedgerError>;
    fn prove_memo(&self, memo: &Self::Memo) -> Result<Self::MerklePath, LedgerError>;

    fn verify_cm(
        parameters: &Self::MerkleParameters,
        digest: &Self::MerkleTreeDigest,
        cm: &Self::Commitment,
        witness: &Self::MerklePath,
    ) -> bool;

    fn verify_sn(
        parameters: &Self::MerkleParameters,
        digest: &Self::MerkleTreeDigest,
        sn: &Self::SerialNumber,
        witness: &Self::MerklePath,
    ) -> bool;

    fn verify_memo(
        parameters: &Self::MerkleParameters,
        digest: &Self::MerkleTreeDigest,
        memo: &Self::Memo,
        witness: &Self::MerklePath,
    ) -> bool;

    fn blocks(&self) -> &Vec<Self::Block>;
}
