use crate::dpc::{Block, Transaction};

use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTreeDigest};
use snarkos_errors::dpc::LedgerError;

use rand::Rng;

pub mod ideal_ledger;
pub use self::ideal_ledger::*;

pub mod ledger;
pub use self::ledger::*;

pub trait Ledger: Sized {
    type Parameters: MerkleParameters;

    type Commitment;
    type SerialNumber;
    type Memo;

    type Transaction: Transaction;

    fn setup<R: Rng>(rng: &mut R) -> Result<Self::Parameters, LedgerError>;

    /// Creates an empty ledger
    fn new(
        parameters: Self::Parameters,
        dummy_cm: Self::Commitment,
        dummy_sn: Self::SerialNumber,
        dummy_memo: Self::Memo,
        dummy_predicate_vk_bytes: Vec<u8>,
        dummy_genesis_address_pair_bytes: Vec<u8>,
    ) -> Result<Self, LedgerError>;

    /// Return the current number of transactions on the ledger.
    fn len(&self) -> usize;

    /// Return the parameters used to construct the ledger data structure.
    fn parameters(&self) -> &Self::Parameters;

    /// Append a (valid) transaction tx to the ledger.
    fn push(&mut self, transaction: Self::Transaction) -> Result<(), LedgerError>;

    /// Return a short digest of the current state of the transaction set data
    /// structure.
    fn digest(&self) -> Option<MerkleTreeDigest<Self::Parameters>>;

    /// Check that st_{ts} is a valid digest for some (past) ledger state.
    fn validate_digest(&self, digest: &MerkleTreeDigest<Self::Parameters>) -> bool;

    fn contains_cm(&self, cm: &Self::Commitment) -> bool;
    fn contains_sn(&self, sn: &Self::SerialNumber) -> bool;
    fn contains_memo(&self, memo: &Self::Memo) -> bool;

    fn prove_cm(&self, cm: &Self::Commitment) -> Result<MerklePath<Self::Parameters>, LedgerError>;
    fn prove_sn(&self, sn: &Self::SerialNumber) -> Result<MerklePath<Self::Parameters>, LedgerError>;
    fn prove_memo(&self, memo: &Self::Memo) -> Result<MerklePath<Self::Parameters>, LedgerError>;

    fn verify_cm(
        parameters: &Self::Parameters,
        digest: &MerkleTreeDigest<Self::Parameters>,
        cm: &Self::Commitment,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool;

    fn verify_sn(
        parameters: &Self::Parameters,
        digest: &MerkleTreeDigest<Self::Parameters>,
        sn: &Self::SerialNumber,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool;

    fn verify_memo(
        parameters: &Self::Parameters,
        digest: &MerkleTreeDigest<Self::Parameters>,
        memo: &Self::Memo,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool;

    fn blocks(&self) -> &Vec<Block<Self::Transaction>>;
}
