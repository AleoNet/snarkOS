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

use super::*;

use crate::PuzzleCommitment;
use std::borrow::Cow;

impl<N: Network, C: ConsensusStorage<N>> Ledger<N, C> {
    /// Returns the block height that contains the given `state root`.
    pub fn find_block_height_from_state_root(&self, state_root: N::StateRoot) -> Result<Option<u32>> {
        self.vm.block_store().find_block_height_from_state_root(state_root)
    }

    /// Returns the block hash that contains the given `transaction ID`.
    pub fn find_block_hash(&self, transaction_id: &N::TransactionID) -> Result<Option<N::BlockHash>> {
        self.vm.block_store().find_block_hash(transaction_id)
    }

    /// Returns the block hash that contains the given `puzzle commitment`.
    pub fn find_block_hash_from_puzzle_commitment(
        &self,
        puzzle_commitment: &PuzzleCommitment<N>,
    ) -> Result<Option<N::BlockHash>> {
        self.vm.block_store().find_block_hash_from_puzzle_commitment(puzzle_commitment)
    }

    /// Returns the transaction ID that contains the given `program ID`.
    pub fn find_deployment_id(&self, program_id: &ProgramID<N>) -> Result<Option<N::TransactionID>> {
        self.vm.transaction_store().find_deployment_id(program_id)
    }

    /// Returns the transaction ID that contains the given `transition ID`.
    pub fn find_transaction_id(&self, transition_id: &N::TransitionID) -> Result<Option<N::TransactionID>> {
        self.vm.transaction_store().find_transaction_id(transition_id)
    }

    /// Returns the transition ID that contains the given `input ID` or `output ID`.
    pub fn find_transition_id(&self, id: &Field<N>) -> Result<N::TransitionID> {
        self.vm.transition_store().find_transition_id(id)
    }

    /// Returns the record ciphertexts that belong to the given view key.
    pub fn find_record_ciphertexts<'a>(
        &'a self,
        view_key: &'a ViewKey<N>,
        filter: RecordsFilter<N>,
    ) -> Result<impl '_ + Iterator<Item = (Field<N>, Cow<'_, Record<N, Ciphertext<N>>>)>> {
        // Derive the x-coordinate of the address corresponding to the given view key.
        let address_x_coordinate = view_key.to_address().to_x_coordinate();
        // Derive the `sk_tag` from the graph key.
        let sk_tag = match GraphKey::try_from(view_key) {
            Ok(graph_key) => graph_key.sk_tag(),
            Err(e) => bail!("Failed to derive the graph key from the view key: {e}"),
        };

        Ok(self.records().flat_map(move |cow| {
            // Retrieve the commitment and record.
            let (commitment, record) = match cow {
                (Cow::Borrowed(commitment), record) => (*commitment, record),
                (Cow::Owned(commitment), record) => (commitment, record),
            };

            // Determine whether to decrypt this record (or not), based on the filter.
            let commitment = match filter {
                RecordsFilter::All => Ok(Some(commitment)),
                RecordsFilter::Spent => Record::<N, Plaintext<N>>::tag(sk_tag, commitment).and_then(|tag| {
                    // Determine if the record is spent.
                    self.contains_tag(&tag).map(|is_spent| match is_spent {
                        true => Some(commitment),
                        false => None,
                    })
                }),
                RecordsFilter::Unspent => Record::<N, Plaintext<N>>::tag(sk_tag, commitment).and_then(|tag| {
                    // Determine if the record is spent.
                    self.contains_tag(&tag).map(|is_spent| match is_spent {
                        true => None,
                        false => Some(commitment),
                    })
                }),
                RecordsFilter::SlowSpent(private_key) => {
                    Record::<N, Plaintext<N>>::serial_number(private_key, commitment).and_then(|serial_number| {
                        // Determine if the record is spent.
                        self.contains_serial_number(&serial_number).map(|is_spent| match is_spent {
                            true => Some(commitment),
                            false => None,
                        })
                    })
                }
                RecordsFilter::SlowUnspent(private_key) => {
                    Record::<N, Plaintext<N>>::serial_number(private_key, commitment).and_then(|serial_number| {
                        // Determine if the record is spent.
                        self.contains_serial_number(&serial_number).map(|is_spent| match is_spent {
                            true => None,
                            false => Some(commitment),
                        })
                    })
                }
            };

            match commitment {
                Ok(Some(commitment)) => {
                    match record.is_owner_with_address_x_coordinate(view_key, &address_x_coordinate) {
                        true => Some((commitment, record)),
                        false => None,
                    }
                }
                Ok(None) => None,
                Err(e) => {
                    warn!("Failed to process 'find_record_ciphertexts({:?})': {e}", filter);
                    None
                }
            }
        }))
    }

    /// Returns the records that belong to the given view key.
    pub fn find_records<'a>(
        &'a self,
        view_key: &'a ViewKey<N>,
        filter: RecordsFilter<N>,
    ) -> Result<impl '_ + Iterator<Item = (Field<N>, Record<N, Plaintext<N>>)>> {
        self.find_record_ciphertexts(view_key, filter).map(|iter| {
            iter.flat_map(|(commitment, record)| match record.decrypt(view_key) {
                Ok(record) => Some((commitment, record)),
                Err(e) => {
                    warn!("Failed to decrypt the record: {e}");
                    None
                }
            })
        })
    }
}
