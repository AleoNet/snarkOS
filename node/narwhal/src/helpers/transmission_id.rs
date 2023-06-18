// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use snarkvm::{
    console::prelude::{error, FromBytes, FromBytesDeserializer, Network, ToBytes, ToBytesSerializer},
    prelude::{PuzzleCommitment, Ratify},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{self, Display, Formatter},
    io::{Read, Result as IoResult, Write},
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TransmissionID<N: Network> {
    /// A ratification.
    Ratification,
    /// A prover solution.
    Solution(PuzzleCommitment<N>),
    /// A transaction.
    Transaction(N::TransactionID),
}

impl<N: Network> From<PuzzleCommitment<N>> for TransmissionID<N> {
    /// Converts the puzzle commitment into an transmission ID.
    fn from(puzzle_commitment: PuzzleCommitment<N>) -> Self {
        Self::Solution(puzzle_commitment)
    }
}

impl<N: Network> From<&N::TransactionID> for TransmissionID<N> {
    /// Converts the transaction ID into an transmission ID.
    fn from(transaction_id: &N::TransactionID) -> Self {
        Self::Transaction(*transaction_id)
    }
}

impl<N: Network> Display for TransmissionID<N> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ratification => write!(f, "ratification"),
            Self::Solution(id) => write!(f, "{}", id),
            Self::Transaction(id) => write!(f, "{}", id),
        }
    }
}

impl<N: Network> FromBytes for TransmissionID<N> {
    /// Reads the transmission ID from the buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the variant.
        let variant = u8::read_le(&mut reader)?;
        // Match the variant.
        match variant {
            0 => Ok(Self::Ratification),
            1 => Ok(Self::Solution(FromBytes::read_le(&mut reader)?)),
            2 => Ok(Self::Transaction(FromBytes::read_le(&mut reader)?)),
            3.. => Err(error("Invalid worker transmission ID variant")),
        }
    }
}

impl<N: Network> ToBytes for TransmissionID<N> {
    /// Writes the transmission ID to the buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the transmission.
        match self {
            Self::Ratification => 0u8.write_le(&mut writer),
            Self::Solution(id) => {
                1u8.write_le(&mut writer)?;
                id.write_le(&mut writer)
            }
            Self::Transaction(id) => {
                2u8.write_le(&mut writer)?;
                id.write_le(&mut writer)
            }
        }
    }
}

impl<N: Network> Serialize for TransmissionID<N> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ToBytesSerializer::serialize_with_size_encoding(self, serializer)
    }
}

impl<'de, N: Network> Deserialize<'de> for TransmissionID<N> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "transmission ID")
    }
}
