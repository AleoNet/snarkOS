use crate::{
    dpc::{DPCTransactions, Transaction},
    BlockHeader,
};

use snarkos_errors::objects::BlockError;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    variable_length_integer::variable_length_integer,
};

use std::io::{Read, Result as IoResult, Write};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Block<T: Transaction> {
    /// First 84 bytes of the block as defined by the encoding used by
    /// "block" messages.
    pub header: BlockHeader,
    /// The block transactions.
    pub transactions: DPCTransactions<T>,
}

impl<T: Transaction> ToBytes for Block<T> {
    #[inline]
    fn write<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.header.write(&mut writer)?;
        self.transactions.write(&mut writer)
    }
}

impl<T: Transaction> FromBytes for Block<T> {
    #[inline]
    fn read<R: Read>(mut reader: R) -> IoResult<Self> {
        let header: BlockHeader = FromBytes::read(&mut reader)?;
        let transactions: DPCTransactions<T> = FromBytes::read(&mut reader)?;

        Ok(Self { header, transactions })
    }
}

impl<T: Transaction> Block<T> {
    pub fn serialize(&self) -> Result<Vec<u8>, BlockError> {
        let mut serialization = vec![];
        serialization.extend(&self.header.serialize().to_vec());
        serialization.extend(&variable_length_integer(self.transactions.len() as u64));

        for transaction in self.transactions.iter() {
            serialization.extend(to_bytes![transaction]?)
        }

        Ok(serialization)
    }

    pub fn deserialize(bytes: &Vec<u8>) -> Result<Self, BlockError> {
        let (header_bytes, transactions_bytes) = bytes.split_at(84);

        let mut header_array: [u8; 84] = [0u8; 84];
        header_array.copy_from_slice(&header_bytes[0..84]);
        let header = BlockHeader::deserialize(&header_array);

        let transactions: DPCTransactions<T> = FromBytes::read(transactions_bytes)?;

        Ok(Block { header, transactions })
    }
}
