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

use crate::{
    state::ledger::Metadata,
    storage::{DataID, DataMap, MapRead, MapReadWrite, Storage, StorageAccess, StorageReadWrite},
};
use snarkvm::{
    compiler::{Deployment, Execution, Transaction, Transition},
    console::{
        program::ProgramID,
        types::field::{Field, Zero},
    },
    prelude::*,
};

use anyhow::{anyhow, Result};

#[derive(Clone, Debug)]
#[allow(clippy::type_complexity)]
pub(crate) struct TransactionState<N: Network, SA: StorageAccess> {
    // TODO (raychu86): Add support for deploy transactions.
    /// Map of transaction_id to (ledger_root, transition ids, edition, metadata)
    transactions: DataMap<N::TransactionID, (Field<N>, Vec<N::TransitionID>, u16, Metadata<N>), SA>,
    /// Map of transition_id to (transaction_id, index, transition)
    transitions: DataMap<N::TransitionID, (N::TransactionID, u8, Transition<N>), SA>,
    /// Map of serial_number to transition_id
    serial_numbers: DataMap<Field<N>, N::TransitionID, SA>,
    /// Map of commitment to transition_id
    commitments: DataMap<Field<N>, N::TransitionID, SA>,
    /// Map of the transaction_id to (program_id, edition, metadata)
    deployments: DataMap<N::TransactionID, (ProgramID<N>, u16, Metadata<N>), SA>,
    /// Map of the program id to the deployed programs.
    programs: DataMap<ProgramID<N>, Deployment<N>, SA>,
}

impl<N: Network, SA: StorageAccess> TransactionState<N, SA> {
    /// Initializes a new instance of `TransactionState`.
    pub(crate) fn open<S: Storage<Access = SA>>(storage: S) -> Result<Self> {
        Ok(Self {
            transactions: storage.open_map(DataID::Transactions)?,
            transitions: storage.open_map(DataID::Transitions)?,
            serial_numbers: storage.open_map(DataID::SerialNumbers)?,
            commitments: storage.open_map(DataID::Commitments)?,
            deployments: storage.open_map(DataID::Deployments)?,
            programs: storage.open_map(DataID::Programs)?,
        })
    }

    /// Returns `true` if the given transaction ID exists in storage.
    pub(crate) fn contains_transaction(&self, transaction_id: &N::TransactionID) -> Result<bool> {
        Ok(self.transactions.contains_key(transaction_id)? || self.deployments.contains_key(transaction_id)?)
    }

    /// Returns `true` if the given serial number exists in storage.
    pub(crate) fn contains_serial_number(&self, serial_number: &Field<N>) -> Result<bool> {
        self.serial_numbers.contains_key(serial_number)
    }

    /// Returns `true` if the given commitment exists in storage.
    pub(crate) fn contains_commitment(&self, commitment: &Field<N>) -> Result<bool> {
        self.commitments.contains_key(commitment)
    }

    // /// Returns the record ciphertext for a given commitment.
    // fn get_ciphertext(&self, commitment: &N::Commitment) -> Result<N::RecordCiphertext> {
    //     // Retrieve the transition ID.
    //     let transition_id = match self.commitments.get(commitment)? {
    //         Some(transition_id) => transition_id,
    //         None => return Err(anyhow!("Commitment {} does not exist in storage", commitment)),
    //     };
    //
    //     // Retrieve the transition.
    //     let transition = match self.transitions.get(&transition_id)? {
    //         Some((_, _, transition)) => transition,
    //         None => return Err(anyhow!("Transition {} does not exist in storage", transition_id)),
    //     };
    //
    //     // Retrieve the ciphertext.
    //     for (candidate_commitment, candidate_ciphertext) in transition.commitments().zip_eq(transition.ciphertexts()) {
    //         if candidate_commitment == commitment {
    //             return Ok(candidate_ciphertext.clone());
    //         }
    //     }
    //
    //     Err(anyhow!("Commitment {} is missing in storage", commitment))
    // }

    /// Returns the transition for a given transition ID.
    pub(crate) fn get_transition(&self, transition_id: &Field<N>) -> Result<Transition<N>> {
        match self.transitions.get(transition_id)? {
            Some((_, _, transition)) => Ok(transition),
            None => Err(anyhow!("Transition {} does not exist in storage", transition_id)),
        }
    }

    /// Returns the transaction for a given transaction ID.
    pub(crate) fn get_transaction(&self, transaction_id: &N::TransactionID) -> Result<Transaction<N>> {
        if let Some((_ledger_root, transition_ids, _, _)) = self.transactions.get(transaction_id)? {
            // Retrieve the transitions.
            let mut execution = Execution::new();
            for transition_id in transition_ids.iter() {
                match self.transitions.get(transition_id)? {
                    Some((_, _, transition)) => execution.push(transition),
                    None => return Err(anyhow!("Transition {} missing in storage", transition_id)),
                };
            }

            Transaction::from_execution(execution, None)
        } else if let Some((program_id, edition, _)) = self.deployments.get(transaction_id)? {
            // Retrieve the program
            let deployment = match self.programs.get(&program_id)? {
                Some(program) => program,
                None => return Err(anyhow!("Program {} missing in storage", program_id)),
            };

            // TODO (raychu86): Include the fees.
            // Transaction::from_deployment(deployment)

            Err(anyhow!(
                "Deployment transaction {transaction_id} not currently supported in storage"
            ))
        } else {
            Err(anyhow!("Transaction {} does not exist in storage", transaction_id))
        }
    }

    /// Returns the transaction metadata for a given transaction ID.
    pub(crate) fn get_transaction_metadata(&self, transaction_id: &N::TransactionID) -> Result<Metadata<N>> {
        if let Some((_, _, _, metadata)) = self.transactions.get(transaction_id)? {
            Ok(metadata)
        } else if let Some((_, _, metadata)) = self.deployments.get(transaction_id)? {
            Ok(metadata)
        } else {
            Err(anyhow!("Transaction {} does not exist in storage", transaction_id))
        }
    }
}

impl<N: Network, SA: StorageReadWrite> TransactionState<N, SA> {
    /// Adds the given transaction to storage.
    pub(crate) fn add_transaction(&self, transaction: &Transaction<N>, metadata: Metadata<N>, batch: Option<usize>) -> Result<()> {
        let transaction_id = transaction.id();

        if self.transactions.contains_key(&transaction_id)? {
            Err(anyhow!("Transaction {} already exists in storage", transaction_id))
        } else {
            match transaction {
                Transaction::Deploy(_, deployment, _) => {
                    let program_id = deployment.program().id();
                    self.deployments
                        .insert(&transaction_id, &(*program_id, deployment.edition(), metadata), batch)?;
                    self.programs.insert(program_id, deployment, batch)?;
                }
                Transaction::Execute(_, execution, _) => {
                    let transition_ids = execution.iter().map(|transition| transition.id()).cloned().collect::<Vec<_>>();

                    // TODO (raychu86) Use a real ledger root.
                    let ledger_root = Field::<N>::zero();

                    // Insert the transaction ID.
                    self.transactions.insert(
                        &transaction_id,
                        &(ledger_root, transition_ids, execution.edition(), metadata),
                        batch,
                    )?;

                    for (i, transition) in execution.iter().enumerate() {
                        let transition_id = transition.id();

                        // Insert the transition.
                        self.transitions
                            .insert(&transition_id, &(transaction_id, i as u8, transition.clone()), batch)?;

                        // Insert the serial numbers.
                        for serial_number in transition.serial_numbers() {
                            self.serial_numbers.insert(serial_number, &transition_id, batch)?;
                        }
                        // Insert the commitments.
                        for commitment in transition.commitments() {
                            self.commitments.insert(commitment, &transition_id, batch)?;
                        }
                    }
                }
            }
            Ok(())
        }
    }

    /// Removes the given transaction ID from storage.
    pub(crate) fn remove_transaction(&self, transaction_id: &N::TransactionID, batch: Option<usize>) -> Result<()> {
        // Remove an execute transaction.
        if let Some((_, transition_ids, _, _)) = self.transactions.get(transaction_id)? {
            // Remove the transaction entry.
            self.transactions.remove(transaction_id, batch)?;

            for transition_id in transition_ids.iter() {
                // Retrieve the transition from the transition ID.
                let transition = match self.transitions.get(transition_id)? {
                    Some((_, _, transition)) => transition,
                    None => return Err(anyhow!("Transition {} missing from transitions map", transition_id)),
                };

                // Remove the transition.
                self.transitions.remove(transition_id, batch)?;

                // Remove the serial numbers.
                for serial_number in transition.serial_numbers() {
                    self.serial_numbers.remove(serial_number, batch)?;
                }
                // Remove the commitments.
                for commitment in transition.commitments() {
                    self.commitments.remove(commitment, batch)?;
                }
            }
        // Remove a deployment transaction
        } else if let Some((program_id, _, _)) = self.deployments.get(transaction_id)? {
            // Remove the deployment entry.
            self.deployments.remove(transaction_id, batch)?;

            // Remove the program.
            self.programs.remove(&program_id, batch)?;
        } else {
            return Err(anyhow!("Transaction {} does not exist in storage", transaction_id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        state::ledger::test_helpers::{sample_genesis_block, CurrentNetwork},
        storage::{
            rocksdb::{tests::temp_dir, RocksDB},
            ReadWrite,
            Storage,
        },
    };

    #[test]
    fn test_open_transaction_state() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let _transaction_state = TransactionState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open transaction state");
    }

    #[test]
    fn test_insert_and_contains_transaction() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let transaction_state = TransactionState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open transaction state");

        let transaction = (*sample_genesis_block().transactions())[0].clone();

        // Insert the transaction
        let metadata = Metadata::<CurrentNetwork>::new(0, Default::default(), 0, 0);
        transaction_state
            .add_transaction(&transaction, metadata, None)
            .expect("Failed to add transaction");

        // Check that the transaction is in storage.
        assert!(transaction_state.contains_transaction(&transaction.id()).unwrap());

        // Check that each commitment is accounted for.
        for commitment in transaction.commitments() {
            assert!(transaction_state.contains_commitment(commitment).unwrap());
        }

        // Check that each serial number is accounted for.
        for serial_number in transaction.serial_numbers() {
            assert!(transaction_state.contains_serial_number(serial_number).unwrap());
        }
    }

    #[test]
    fn test_insert_and_get_transaction() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let transaction_state = TransactionState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open transaction state");

        let transaction = (*sample_genesis_block().transactions())[0].clone();

        // Insert the transaction
        let metadata = Metadata::<CurrentNetwork>::new(0, Default::default(), 0, 0);
        transaction_state
            .add_transaction(&transaction, metadata, None)
            .expect("Failed to add transaction");

        // Assert that the transaction in storage is the same.
        let stored_transaction = transaction_state.get_transaction(&transaction.id()).unwrap();
        assert_eq!(transaction, stored_transaction);
    }

    #[test]
    fn test_insert_and_remove_transaction() {
        let storage = RocksDB::<ReadWrite>::open(temp_dir(), 0).expect("Failed to open storage");
        let transaction_state = TransactionState::<CurrentNetwork, ReadWrite>::open(storage).expect("Failed to open transaction state");

        let transaction = (*sample_genesis_block().transactions())[0].clone();
        let transaction_id = transaction.id();

        // Insert the transaction
        let metadata = Metadata::<CurrentNetwork>::new(0, Default::default(), 0, 0);
        transaction_state
            .add_transaction(&transaction, metadata, None)
            .expect("Failed to add transaction");
        assert!(transaction_state.contains_transaction(&transaction_id).unwrap());

        // Remove the transaction.
        transaction_state
            .remove_transaction(&transaction_id, None)
            .expect("Failed to remove transaction");
        assert!(!transaction_state.contains_transaction(&transaction_id).unwrap());
    }
}
