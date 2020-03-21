use crate::dpc::Transaction;
use snarkos_algorithms::merkle_tree::{MerkleParameters, MerklePath, MerkleTreeDigest};
use snarkos_errors::dpc::LedgerError;

use rand::Rng;

pub mod block;
pub use self::block::*;

pub mod ideal_ledger;
pub use self::ideal_ledger::*;

pub mod transactions;
pub use self::transactions::*;

pub type MerkleTreeParameters<P> = <P as MerkleParameters>::H;

pub trait Ledger {
    type Parameters: MerkleParameters;

    type Commitment;
    type SerialNumber;
    type Memo;

    type Transaction: Transaction;

    fn setup<R: Rng>(rng: &mut R) -> Result<MerkleTreeParameters<Self::Parameters>, LedgerError>;

    /// Creates an empty ledger
    fn new(
        parameters: MerkleTreeParameters<Self::Parameters>,
        dummy_cm: Self::Commitment,
        dummy_sn: Self::SerialNumber,
        dummy_memo: Self::Memo,
    ) -> Self;

    /// Return the current number of transactions on the ledger.
    fn len(&self) -> usize;

    /// Return the parameters used to construct the ledger data structure.
    fn parameters(&self) -> &MerkleTreeParameters<Self::Parameters>;

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
        parameters: &MerkleTreeParameters<Self::Parameters>,
        digest: &MerkleTreeDigest<Self::Parameters>,
        cm: &Self::Commitment,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool;

    fn verify_sn(
        parameters: &MerkleTreeParameters<Self::Parameters>,
        digest: &MerkleTreeDigest<Self::Parameters>,
        sn: &Self::SerialNumber,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool;

    fn verify_memo(
        parameters: &MerkleTreeParameters<Self::Parameters>,
        digest: &MerkleTreeDigest<Self::Parameters>,
        memo: &Self::Memo,
        witness: &MerklePath<Self::Parameters>,
    ) -> bool;
}
