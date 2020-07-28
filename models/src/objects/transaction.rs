use snarkos_errors::objects::TransactionError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::hash::Hash;

pub trait Transaction: Clone + Eq + FromBytes + ToBytes {
    type Commitment: Clone + Eq + Hash + FromBytes + ToBytes;
    type Digest: Clone + Eq + Hash + FromBytes + ToBytes;
    type LocalDataCommitment: Clone + Eq + Hash + FromBytes + ToBytes;
    type Memorandum: Clone + Eq + Hash + FromBytes + ToBytes;
    type PredicateCommitment: Clone + Eq + Hash + FromBytes + ToBytes;
    type SerialNumber: Clone + Eq + Hash + FromBytes + ToBytes;
    type EncryptedRecord: Clone + Eq + FromBytes + ToBytes;

    /// Returns the transaction identifier.
    fn transaction_id(&self) -> Result<[u8; 32], TransactionError>;

    /// Returns the network_id in the transaction.
    fn network_id(&self) -> u8;

    /// Returns the ledger digest.
    fn ledger_digest(&self) -> &Self::Digest;

    /// Returns the old serial numbers.
    fn old_serial_numbers(&self) -> &[Self::SerialNumber];

    /// Returns the new commitments.
    fn new_commitments(&self) -> &[Self::Commitment];

    /// Returns the predicate commitment in the transaction.
    fn predicate_commitment(&self) -> &Self::PredicateCommitment;

    /// Returns the local data commitment in the transaction.
    fn local_data_commitment(&self) -> &Self::LocalDataCommitment;

    /// Returns the value balance in the transaction.
    fn value_balance(&self) -> i64;

    /// Returns the encrypted records
    fn encrypted_records(&self) -> &[Self::EncryptedRecord];

    /// Returns the memorandum.
    fn memorandum(&self) -> &Self::Memorandum;

    /// Returns the transaction size in bytes.
    fn size(&self) -> usize;
}
