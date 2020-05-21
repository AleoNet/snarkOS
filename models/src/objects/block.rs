use crate::objects::TransactionScheme;
use snarkos_utilities::bytes::{FromBytes, ToBytes};

pub trait BlockScheme: Clone + Eq + FromBytes + ToBytes {
    type BlockHeader: Clone + Eq + FromBytes + ToBytes;
    type Transaction: TransactionScheme;

    /// Returns the header.
    fn header(&self) -> &Self::BlockHeader;

    /// Returns the transactions.
    fn transactions(&self) -> &[Self::Transaction];
}
