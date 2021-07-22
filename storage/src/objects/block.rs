// Copyright (C) 2019-2021 Aleo Systems Inc.
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
use anyhow::*;
use snarkvm_dpc::{
    testnet1::{instantiated::Components, Transaction},
    Block,
    TransactionScheme,
};
use snarkvm_utilities::{read_variable_length_integer, variable_length_integer, FromBytes, Read, ToBytes, Write};
use std::io::Result as IoResult;

use crate::{SerialBlockHeader, SerialTransaction, VMTransaction};

pub trait VMBlock: Sized {
    fn deserialize(tx: &SerialBlock) -> IoResult<Self>;

    fn serialize(&self) -> Result<SerialBlock>;
}

impl<T: TransactionScheme + VMTransaction> VMBlock for Block<T> {
    fn deserialize(_tx: &SerialBlock) -> IoResult<Self> {
        unimplemented!()
    }

    fn serialize(&self) -> Result<SerialBlock> {
        Ok(SerialBlock {
            header: self.header.clone().into(),
            transactions: self
                .transactions
                .0
                .iter()
                .map(|x| x.serialize())
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialBlock {
    pub header: SerialBlockHeader,
    pub transactions: Vec<SerialTransaction>,
}

impl SerialBlock {
    pub fn serialize(&self) -> Vec<u8> {
        let mut out = vec![];
        self.write_le(&mut out).expect("failed to serialize block");
        out
    }

    pub fn write_transactions<W: Write>(&self, mut writer: W) -> IoResult<()> {
        variable_length_integer(self.transactions.len() as u64).write_le(&mut writer)?;

        for transaction in &self.transactions {
            transaction.write_le(&mut writer)?;
        }
        Ok(())
    }

    pub fn read_transactions<R: Read>(mut reader: R) -> Result<Vec<SerialTransaction>> {
        //todo: use protobuf
        let num_transactions = read_variable_length_integer(&mut reader)?;
        let mut transactions = Vec::with_capacity(num_transactions);
        for _ in 0..num_transactions {
            let transaction: Transaction<Components> = FromBytes::read_le(&mut reader)?;
            transactions.push(transaction.serialize()?);
        }
        Ok(transactions)
    }
}

impl ToBytes for SerialBlock {
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        self.header.write_le(&mut writer)?;
        self.write_transactions(writer)
    }
}
