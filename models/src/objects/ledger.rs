use crate::objects::{BlockScheme, Transaction};
use snarkos_errors::dpc::LedgerError;

use std::path::PathBuf;

pub trait LedgerScheme: Sized {
    type Block: BlockScheme;
    type Commitment;
    type Memo;
    type MerkleParameters;
    type MerklePath;
    type MerkleTreeDigest;
    type SerialNumber;
    type Transaction: Transaction;

    /// Instantiates a new ledger with a genesis block.
    fn new(path: &PathBuf, parameters: Self::MerkleParameters, genesis_block: Self::Block)
    -> Result<Self, LedgerError>;

    /// Returns the number of blocks including the genesis block
    fn len(&self) -> usize;

    /// Return the parameters used to construct the ledger Merkle tree.
    fn parameters(&self) -> &Self::MerkleParameters;

    /// Return a digest of the latest ledger Merkle tree.
    fn digest(&self) -> Option<Self::MerkleTreeDigest>;

    /// Check that st_{ts} is a valid digest for some (past) ledger state.
    fn validate_digest(&self, digest: &Self::MerkleTreeDigest) -> bool;

    /// Returns true if the given commitment exists in the ledger.
    fn contains_cm(&self, cm: &Self::Commitment) -> bool;

    /// Returns true if the given serial number exists in the ledger.
    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool;

    /// Returns true if the given memo exists in the ledger.
    fn contains_memo(&self, memo: &Self::Memo) -> bool;

    /// Returns the Merkle path to the latest ledger digest
    /// for a given commitment, if it exists in the ledger.
    fn prove_cm(&self, cm: &Self::Commitment) -> Result<Self::MerklePath, LedgerError>;

    /// Returns true if the given Merkle path is a valid witness for
    /// the given ledger digest and commitment.
    fn verify_cm(
        parameters: &Self::MerkleParameters,
        digest: &Self::MerkleTreeDigest,
        cm: &Self::Commitment,
        witness: &Self::MerklePath,
    ) -> bool;
}
