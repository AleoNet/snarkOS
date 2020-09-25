// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{dpc::DPCTransactions, BlockHeader};
use snarkos_errors::objects::BlockError;
use snarkos_models::objects::{BlockScheme, Transaction};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    variable_length_integer::variable_length_integer,
};

use std::io::{Read, Result as IoResult, Write};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Block<T: Transaction> {
    /// First `HEADER_SIZE` bytes of the block as defined by the encoding used by
    /// "block" messages.
    pub header: BlockHeader,
    /// The block transactions.
    pub transactions: DPCTransactions<T>,
}

impl<T: Transaction> BlockScheme for Block<T> {
    type BlockHeader = BlockHeader;
    type Transaction = T;

    /// Returns the header.
    fn header(&self) -> &Self::BlockHeader {
        &self.header
    }

    /// Returns the transactions.
    fn transactions(&self) -> &[Self::Transaction] {
        self.transactions.as_slice()
    }
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

    pub fn deserialize(bytes: &[u8]) -> Result<Self, BlockError> {
        const HEADER_SIZE: usize = BlockHeader::size();
        let (header_bytes, transactions_bytes) = bytes.split_at(HEADER_SIZE);

        let mut header_array = [0u8; HEADER_SIZE];
        header_array.copy_from_slice(&header_bytes[0..HEADER_SIZE]);
        let header = BlockHeader::deserialize(&header_array);

        let transactions: DPCTransactions<T> = FromBytes::read(transactions_bytes)?;

        Ok(Block { header, transactions })
    }
}
