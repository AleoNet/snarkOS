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

use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::{error, FromBytes, FromBytesDeserializer, Network, ToBytes, ToBytesSerializer},
    prelude::{block::Transaction, coinbase::ProverSolution},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::io::{Read, Result as IoResult, Write};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transmission<N: Network> {
    /// A ratification.
    Ratification,
    /// A prover solution.
    Solution(Data<ProverSolution<N>>),
    /// A transaction.
    Transaction(Data<Transaction<N>>),
}

impl<N: Network> From<ProverSolution<N>> for Transmission<N> {
    /// Converts the prover solution into an transmission.
    fn from(solution: ProverSolution<N>) -> Self {
        Self::Solution(Data::Object(solution))
    }
}

impl<N: Network> From<Transaction<N>> for Transmission<N> {
    /// Converts the transaction into an transmission.
    fn from(transaction: Transaction<N>) -> Self {
        Self::Transaction(Data::Object(transaction))
    }
}

impl<N: Network> From<Data<ProverSolution<N>>> for Transmission<N> {
    /// Converts the prover solution into an transmission.
    fn from(solution: Data<ProverSolution<N>>) -> Self {
        Self::Solution(solution)
    }
}

impl<N: Network> From<Data<Transaction<N>>> for Transmission<N> {
    /// Converts the transaction into an transmission.
    fn from(transaction: Data<Transaction<N>>) -> Self {
        Self::Transaction(transaction)
    }
}

impl<N: Network> FromBytes for Transmission<N> {
    /// Reads the transmission from the buffer.
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u8::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 0 {
            return Err(error("Invalid worker transmission version"));
        }

        // Read the variant.
        let variant = u8::read_le(&mut reader)?;
        // Match the variant.
        match variant {
            0 => Ok(Self::Ratification),
            1 => {
                // Read the prover solution.
                let solution = ProverSolution::read_le(&mut reader)?;
                // Return the prover solution.
                Ok(Self::Solution(Data::Object(solution)))
            }
            2 => {
                // Read the transaction.
                let transaction = Transaction::read_le(&mut reader)?;
                // Return the transaction.
                Ok(Self::Transaction(Data::Object(transaction)))
            }
            3.. => Err(error("Invalid worker transmission variant")),
        }
    }
}

impl<N: Network> ToBytes for Transmission<N> {
    /// Writes the transmission to the buffer.
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        0u8.write_le(&mut writer)?;
        // Write the transmission.
        match self {
            Self::Ratification => 0u8.write_le(&mut writer),
            Self::Solution(solution) => {
                1u8.write_le(&mut writer)?;
                solution.serialize_blocking_into(&mut writer).map_err(|e| error(e.to_string()))
            }
            Self::Transaction(transaction) => {
                2u8.write_le(&mut writer)?;
                transaction.serialize_blocking_into(&mut writer).map_err(|e| error(e.to_string()))
            }
        }
    }
}

impl<N: Network> Serialize for Transmission<N> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        ToBytesSerializer::serialize_with_size_encoding(self, serializer)
    }
}

impl<'de, N: Network> Deserialize<'de> for Transmission<N> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "transmission")
    }
}
