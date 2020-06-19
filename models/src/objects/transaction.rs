use snarkos_errors::objects::TransactionError;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

use std::hash::Hash;

pub trait Transaction: Clone + Eq + FromBytes + ToBytes {
    type Commitment: Clone + Eq + Hash + FromBytes + ToBytes;
    type Memorandum: Clone + Eq + Hash + FromBytes + ToBytes;
    type SerialNumber: Clone + Eq + Hash + FromBytes + ToBytes;

    /// Returns the old serial numbers.
    fn old_serial_numbers(&self) -> &[Self::SerialNumber];

    /// Returns the new commitments.
    fn new_commitments(&self) -> &[Self::Commitment];

    /// Returns the memorandum.
    fn memorandum(&self) -> &Self::Memorandum;

    /// Returns the transaction identifier.
    fn transaction_id(&self) -> Result<[u8; 32], TransactionError>;

    /// Returns the transaction size in bytes
    fn size(&self) -> usize;

    /// Returns the value balance of the transaction
    fn value_balance(&self) -> i64;

    /// Returns the network_id of the transaction
    fn network_id(&self) -> u8;
}
