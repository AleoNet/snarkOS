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

use snarkvm::{
    circuit::Aleo,
    compiler::{Execution, Process, Program, Transition, VerifyingKey},
    console::types::Field,
    prelude::*,
};

use core::fmt;
use serde::ser::SerializeStruct;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Transaction<N: Network> {
    /// The transaction deployment publishes an Aleo program to the network.
    Deploy(N::TransactionID, Program<N>, VerifyingKey<N>),
    /// The transaction execution represents a call to an Aleo program.
    Execute(N::TransactionID, Vec<Transition<N>>),
}

impl<N: Network> Transaction<N> {
    /// Initializes a new deployment transaction.
    pub fn deploy(program: Program<N>, verifying_key: VerifyingKey<N>) -> Result<Self> {
        // Compute the transaction ID.
        let id = N::hash_bhp1024(&program.to_bytes_le()?.to_bits_le())?.into();
        // Construct the deploy transaction.
        Ok(Self::Deploy(id, program, verifying_key))
    }

    /// Initializes a new execution transaction.
    pub fn execute(transitions: Vec<Transition<N>>) -> Result<Self> {
        // Ensure the transaction is not empty.
        ensure!(!transitions.is_empty(), "Attempted to create an empty transaction execution");
        // Compute the transaction ID.
        let id_bits: Vec<_> = transitions.iter().flat_map(|transition| transition.id().to_bits_le()).collect();
        let id = N::hash_bhp1024(&id_bits)?.into();
        // Construct the execute transaction.
        Ok(Self::Execute(id, transitions))
    }

    /// Returns `true` if the transaction is valid.
    pub fn is_valid<A: Aleo<Network = N, BaseField = N::Field>>(&self, process: &Process<N, A>) -> bool {
        match self {
            Transaction::Deploy(id, program, _verifying_key) => {
                // Convert the program into bytes.
                let program_bytes = match program.to_bytes_le() {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        warn!("Unable to convert program into bytes for transaction (deploy, {id}): {error}");
                        return false;
                    }
                };

                // Check the transaction ID.
                match N::hash_bhp1024(&program_bytes.to_bits_le()) {
                    Ok(candidate_id) => {
                        // Ensure the transaction ID matches the one in the transaction.
                        if candidate_id != **id {
                            warn!("Transaction ({id}) has an incorrect transaction ID.");
                            return false;
                        }
                    }
                    Err(error) => {
                        warn!("Unable to compute transaction ID for transaction (deploy, {id}): {error}");
                        return false;
                    }
                };

                /// TODO (howardwu): Check the program (1. ensure the program ID does not exist already, 2. check it is well-formed).
                /// TODO (howardwu): Check the verifying key.
                true
            }
            Transaction::Execute(id, transitions) => {
                // Ensure there is at least 1 transition.
                if transitions.is_empty() {
                    warn!("Transaction ({id}) has no transitions.");
                    return false;
                }

                // Check the transaction ID.
                let id_bits: Vec<_> = transitions.iter().flat_map(|transition| transition.id().to_bits_le()).collect();
                match N::hash_bhp1024(&id_bits) {
                    Ok(candidate_id) => {
                        // Ensure the transaction ID matches the one in the transaction.
                        if candidate_id != **id {
                            warn!("Transaction ({id}) has an incorrect transaction ID.");
                            return false;
                        }
                    }
                    Err(error) => {
                        warn!("Unable to compute transaction ID for transaction (execute, {id}): {error}");
                        return false;
                    }
                };

                // Ensure each transition is valid.
                if let Err(error) = process.verify(Execution::from(transitions)) {
                    warn!("Transaction ({id}) is invalid: {error}\n{:#?}", transitions);
                    return false;
                }

                true
            }
        }
    }

    /// Returns the transaction ID.
    pub const fn id(&self) -> N::TransactionID {
        match self {
            Transaction::Deploy(id, ..) => *id,
            Transaction::Execute(id, ..) => *id,
        }
    }

    /// Returns an iterator over the serial numbers, for all executed transition inputs that are records.
    pub fn serial_numbers(&self) -> impl '_ + Iterator<Item = &Field<N>> {
        match self {
            Transaction::Deploy(..) => [].iter().flat_map(Transition::serial_numbers),
            Transaction::Execute(.., transitions) => transitions.iter().flat_map(Transition::serial_numbers),
        }
    }

    /// Returns an iterator over the commitments, for all executed transition outputs that are records.
    pub fn commitments(&self) -> impl '_ + Iterator<Item = &Field<N>> {
        match self {
            Transaction::Deploy(..) => [].iter().flat_map(Transition::commitments),
            Transaction::Execute(.., transitions) => transitions.iter().flat_map(Transition::commitments),
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
        write!(f, "{}", serde_json::to_string(self).map_err::<fmt::Error, _>(ser::Error::custom)?)
    }
}

impl<N: Network> FromBytes for Transaction<N> {
    /// Reads the transaction from the buffer.
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        // Read the version.
        let version = u16::read_le(&mut reader)?;
        // Ensure the version is valid.
        if version != 0 {
            return Err(error("Invalid transaction version"));
        }

        // Read the variant.
        let variant = u8::read_le(&mut reader)?;
        // Match the variant.
        let transaction = match variant {
            0 => {
                // Read the ID.
                let id = N::TransactionID::read_le(&mut reader)?;
                // Read the program.
                let program = Program::read_le(&mut reader)?;
                // Read the verifying key.
                let verifying_key = VerifyingKey::read_le(&mut reader)?;
                // Construct the transaction.
                Transaction::Deploy(id, program, verifying_key)
            }
            1 => {
                // Read the ID.
                let id = N::TransactionID::read_le(&mut reader)?;
                // Read the number of transitions.
                let num_transitions = u16::read_le(&mut reader)?;
                // Ensure the number of transitions is nonzero.
                if num_transitions == 0 {
                    warn!("Transaction (from 'read_le') has no transitions");
                    return Err(error("Transaction (from 'read_le') has no transitions"));
                }
                // Read the transitions.
                let transitions = (0..num_transitions)
                    .map(|_| Transition::read_le(&mut reader))
                    .collect::<IoResult<Vec<_>>>()?;
                // Construct the transaction.
                Transaction::Execute(id, transitions)
            }
            _ => return Err(error("Invalid transaction variant")),
        };
        // Return the transaction.
        Ok(transaction)
    }
}

impl<N: Network> ToBytes for Transaction<N> {
    /// Writes the transaction to the buffer.
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        // Write the version.
        0u16.write_le(&mut writer)?;

        // Write the transaction.
        match self {
            Self::Deploy(id, program, verifying_key) => {
                // Write the variant.
                0u8.write_le(&mut writer)?;
                // Write the ID.
                id.write_le(&mut writer)?;
                // Write the program.
                program.write_le(&mut writer)?;
                // Write the verifying key.
                verifying_key.write_le(&mut writer)
            }
            Self::Execute(id, transitions) => {
                // Write the variant.
                1u8.write_le(&mut writer)?;
                // Write the ID.
                id.write_le(&mut writer)?;
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
            true => match self {
                Self::Deploy(id, program, verifying_key) => {
                    let mut transaction = serializer.serialize_struct("Transaction", 4)?;
                    transaction.serialize_field("type", "deploy")?;
                    transaction.serialize_field("id", &id)?;
                    transaction.serialize_field("program", &program)?;
                    transaction.serialize_field("verifying_key", &verifying_key)?;
                    transaction.end()
                }
                Self::Execute(id, transitions) => {
                    let mut transaction = serializer.serialize_struct("Transaction", 3)?;
                    transaction.serialize_field("type", "execute")?;
                    transaction.serialize_field("id", &id)?;
                    transaction.serialize_field("transitions", &transitions)?;
                    transaction.end()
                }
            },
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for Transaction<N> {
    /// Deserializes the transaction from a JSON-string or buffer.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                // Deserialize the transaction into a JSON value.
                let transaction = serde_json::Value::deserialize(deserializer)?;
                // Retrieve the transaction ID.
                let id: N::TransactionID = serde_json::from_value(transaction["id"].clone()).map_err(de::Error::custom)?;

                // Recover the transaction.
                let transaction = match transaction["type"].as_str() {
                    Some("deploy") => {
                        // Retrieve the program.
                        let program = serde_json::from_value(transaction["program"].clone()).map_err(de::Error::custom)?;
                        // Retrieve the verifying key.
                        let verifying_key = serde_json::from_value(transaction["verifying_key"].clone()).map_err(de::Error::custom)?;
                        // Construct the transaction.
                        Transaction::deploy(program, verifying_key).map_err(de::Error::custom)?
                    }
                    Some("execute") => {
                        // Retrieve the transitions.
                        let transitions = serde_json::from_value(transaction["transitions"].clone()).map_err(de::Error::custom)?;
                        // Construct the transaction.
                        Transaction::execute(transitions).map_err(de::Error::custom)?
                    }
                    _ => return Err(de::Error::custom("Invalid transaction type")),
                };

                // Ensure the transaction ID matches.
                match id == transaction.id() {
                    true => Ok(transaction),
                    false => Err(error("Mismatching transaction ID, possible data corruption")).map_err(de::Error::custom),
                }
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "transaction"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::Block;

    use snarkvm::prelude::Testnet3;

    type CurrentNetwork = Testnet3;
    type A = snarkvm::circuit::AleoV0;

    #[test]
    fn test_transaction_serde_json() {
        let expected_transaction = (*Block::<CurrentNetwork>::genesis::<A>().unwrap().transactions())[0].clone();

        // Serialize
        let expected_string = expected_transaction.to_string();
        let candidate_string = serde_json::to_string(&expected_transaction).unwrap();
        assert_eq!(2670, candidate_string.len(), "Update me if serialization has changed");
        assert_eq!(expected_string, candidate_string);

        // Deserialize
        assert_eq!(
            expected_transaction,
            Transaction::<CurrentNetwork>::from_str(&candidate_string).unwrap()
        );
        assert_eq!(expected_transaction, serde_json::from_str(&candidate_string).unwrap());
    }

    #[test]
    fn test_transaction_bincode() {
        let expected_transaction = (*Block::<CurrentNetwork>::genesis::<A>().unwrap().transactions())[0].clone();

        // Serialize
        let expected_bytes = expected_transaction.to_bytes_le().unwrap();
        let candidate_bytes = bincode::serialize(&expected_transaction).unwrap();
        assert_eq!(1362, expected_bytes.len(), "Update me if serialization has changed");
        // TODO (howardwu): Serialization - Handle the inconsistency between ToBytes and Serialize (off by a length encoding).
        assert_eq!(&expected_bytes[..], &candidate_bytes[8..]);

        // Deserialize
        assert_eq!(
            expected_transaction,
            Transaction::<CurrentNetwork>::read_le(&expected_bytes[..]).unwrap()
        );
        assert_eq!(expected_transaction, bincode::deserialize(&candidate_bytes[..]).unwrap());
    }
}
