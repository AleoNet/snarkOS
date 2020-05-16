use crate::objects::Transaction;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

pub trait BlockScheme: Clone + Eq + FromBytes + ToBytes {
    type BlockHeader: Clone + Eq + FromBytes + ToBytes;
    type Transaction: Transaction;

    /// Returns the header.
    fn header(&self) -> Self::BlockHeader;

    /// Returns the transactions.
    fn transactions(&self) -> &[Self::Transaction];
}
