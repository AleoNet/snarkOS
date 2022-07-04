// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use snarkvm::{compiler::Transition, prelude::*};

use core::fmt;
use serde::ser::SerializeStruct;

#[derive(Clone, PartialEq, Eq)]
pub enum Transaction<N: Network> {
    Deploy(N::TransactionID),
    Execute(Vec<Transition<N>>),
}

impl<N: Network> Transaction<N> {
    /// Returns the transaction ID.
    pub const fn id(&self) -> N::TransactionID {
        match self {
            Transaction::Deploy(id) => *id,
            // Transaction::Execute(transitions) => transitions[0].id(),
            Transaction::Execute(transitions) => todo!(),
        }
    }

    /// Returns `true` if the transaction is valid.
    pub fn is_valid(&self) -> bool {
        match self {
            Transaction::Deploy(_) => true,
            Transaction::Execute(_) => true,
        }
    }
}

impl<N: Network> FromStr for Transaction<N> {
    type Err = anyhow::Error;

    /// Initializes the transaction from a JSON-string.
    fn from_str(transaction: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(transaction)?)
    }
}

impl<N: Network> Display for Transaction<N> {
    /// Displays the transaction as a JSON-string.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).map_err::<fmt::Error, _>(serde::ser::Error::custom)?
        )
    }
}

impl<N: Network> FromBytes for Transaction<N> {
    /// Reads the transaction from the buffer.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the variant.
        let variant = u8::read_le(&mut reader)?;
        // Match the variant.
        match variant {
            0 => {
                // Read the ID.
                let id = N::TransactionID::read_le(&mut reader)?;
                // Construct the transaction.
                Ok(Transaction::Deploy(id))
            }
            1 => {
                // Read the number of transitions.
                let num_transitions = u16::read_le(&mut reader)?;
                // Read the transitions.
                let transitions = (0..num_transitions)
                    .map(|_| Transition::read_le(&mut reader))
                    .collect::<Result<Vec<_>>>()?;
                // Construct the transaction.
                Ok(Transaction::Execute(transitions))
            }
            _ => Err(error("Invalid transaction variant")),
        }
    }
}

impl<N: Network> ToBytes for Transaction<N> {
    /// Writes the transaction to the buffer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        match self {
            Self::Deploy(id) => {
                // Write the variant.
                0u8.write_le(&mut writer)?;
                // Write the ID.
                id.write_le(&mut writer)
            }
            Self::Execute(transitions) => {
                // Write the variant.
                1u8.write_le(&mut writer)?;
                // Write the number of transitions.
                (transitions.len() as u16).write_le(&mut writer)?;
                // Write the transitions.
                transitions.write_le(&mut writer)
            }
        }
    }
}

impl<N: Network> Serialize for Transaction<N> {
    /// Serializes the transaction to a JSON-string or buffer.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut block = serializer.serialize_struct("Transaction", 4)?;
                block.serialize_field("block_hash", &self.block_hash)?;
                block.serialize_field("previous_hash", &self.previous_hash)?;
                block.serialize_field("header", &self.header)?;
                block.serialize_field("transactions", &self.transactions)?;
                block.end()
            }
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for Transaction<N> {
    /// Deserializes the transaction from a JSON-string or buffer.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let block = serde_json::Value::deserialize(deserializer)?;
                let block_hash: N::BlockHash = serde_json::from_value(block["block_hash"].clone()).map_err(de::Error::custom)?;

                // Recover the block.
                let block = Self::from(
                    serde_json::from_value(block["previous_hash"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(block["header"].clone()).map_err(de::Error::custom)?,
                    serde_json::from_value(block["transactions"].clone()).map_err(de::Error::custom)?,
                )
                .map_err(de::Error::custom)?;

                // Ensure the block hash matches.
                match block_hash == block.hash() {
                    true => Ok(block),
                    false => Err(error("Mismatching block hash, possible data corruption")).map_err(de::Error::custom),
                }
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "block"),
        }
    }
}
