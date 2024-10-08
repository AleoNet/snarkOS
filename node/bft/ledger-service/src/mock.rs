// Copyright 2024 Aleo Network Foundation
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

use crate::{fmt_id, LedgerService};
use snarkvm::{
    ledger::{
        block::{Block, Transaction},
        committee::Committee,
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
        puzzle::{Solution, SolutionID},
    },
    prelude::{bail, ensure, Address, Field, Network, Result, Zero},
};

use indexmap::IndexMap;
use parking_lot::Mutex;
use std::{collections::BTreeMap, ops::Range};
use tracing::*;

/// A mock ledger service that always returns `false`.
#[derive(Debug)]
pub struct MockLedgerService<N: Network> {
    committee: Committee<N>,
    height_to_round_and_hash: Mutex<BTreeMap<u32, (u64, N::BlockHash)>>,
}

impl<N: Network> MockLedgerService<N> {
    /// Initializes a new mock ledger service.
    pub fn new(committee: Committee<N>) -> Self {
        Self { committee, height_to_round_and_hash: Default::default() }
    }

    /// Initializes a new mock ledger service at the specified height.
    pub fn new_at_height(committee: Committee<N>, height: u32) -> Self {
        let mut height_to_hash = BTreeMap::new();
        for i in 0..=height {
            height_to_hash.insert(i, (i as u64 * 2, Field::<N>::from_u32(i).into()));
        }
        Self { committee, height_to_round_and_hash: Mutex::new(height_to_hash) }
    }
}

#[async_trait]
impl<N: Network> LedgerService<N> for MockLedgerService<N> {
    /// Returns the latest round in the ledger.
    fn latest_round(&self) -> u64 {
        *self.height_to_round_and_hash.lock().keys().last().unwrap_or(&0) as u64
    }

    /// Returns the latest block height in the canonical ledger.
    fn latest_block_height(&self) -> u32 {
        self.height_to_round_and_hash.lock().last_key_value().map(|(height, _)| *height).unwrap_or(0)
    }

    /// Returns the latest block in the ledger.
    fn latest_block(&self) -> Block<N> {
        unreachable!("MockLedgerService does not support latest_block")
    }

    /// Returns the latest restrictions ID in the ledger.
    fn latest_restrictions_id(&self) -> Field<N> {
        Field::zero()
    }

    /// Returns the latest cached leader and its associated round.
    fn latest_leader(&self) -> Option<(u64, Address<N>)> {
        None
    }

    /// Updates the latest cached leader and its associated round.
    fn update_latest_leader(&self, _round: u64, _leader: Address<N>) {}

    /// Returns `true` if the given block height exists in the canonical ledger.
    fn contains_block_height(&self, height: u32) -> bool {
        self.height_to_round_and_hash.lock().contains_key(&height)
    }

    /// Returns the canonical block height for the given block hash, if it exists.
    fn get_block_height(&self, hash: &N::BlockHash) -> Result<u32> {
        match self
            .height_to_round_and_hash
            .lock()
            .iter()
            .find_map(|(height, (_, h))| if h == hash { Some(*height) } else { None })
        {
            Some(height) => Ok(height),
            None => bail!("Missing block {hash}"),
        }
    }

    /// Returns the canonical block hash for the given block height, if it exists.
    fn get_block_hash(&self, height: u32) -> Result<N::BlockHash> {
        match self.height_to_round_and_hash.lock().get(&height).cloned() {
            Some((_, hash)) => Ok(hash),
            None => bail!("Missing block {height}"),
        }
    }

    /// Returns the block round for the given block height, if it exists.
    fn get_block_round(&self, height: u32) -> Result<u64> {
        match self
            .height_to_round_and_hash
            .lock()
            .iter()
            .find_map(|(h, (round, _))| if *h == height { Some(*round) } else { None })
        {
            Some(round) => Ok(round),
            None => bail!("Missing block {height}"),
        }
    }

    /// Returns the block for the given block height.
    fn get_block(&self, _height: u32) -> Result<Block<N>> {
        unreachable!("MockLedgerService does not support get_block")
    }

    /// Returns the blocks in the given block range.
    /// The range is inclusive of the start and exclusive of the end.
    fn get_blocks(&self, _heights: Range<u32>) -> Result<Vec<Block<N>>> {
        unreachable!("MockLedgerService does not support get_blocks")
    }

    /// Returns the solution for the given solution ID.
    fn get_solution(&self, _solution_id: &SolutionID<N>) -> Result<Solution<N>> {
        unreachable!("MockLedgerService does not support get_solution")
    }

    /// Returns the unconfirmed transaction for the given transaction ID.
    fn get_unconfirmed_transaction(&self, _transaction_id: N::TransactionID) -> Result<Transaction<N>> {
        unreachable!("MockLedgerService does not support get_unconfirmed_transaction")
    }

    /// Returns the batch certificate for the given batch certificate ID.
    fn get_batch_certificate(&self, _certificate_id: &Field<N>) -> Result<BatchCertificate<N>> {
        unreachable!("MockLedgerService does not support get_batch_certificate")
    }

    /// Returns the current committee.
    fn current_committee(&self) -> Result<Committee<N>> {
        Ok(self.committee.clone())
    }

    /// Returns the committee for the given round.
    fn get_committee_for_round(&self, _round: u64) -> Result<Committee<N>> {
        Ok(self.committee.clone())
    }

    /// Returns the committee lookback for the given round.
    fn get_committee_lookback_for_round(&self, _round: u64) -> Result<Committee<N>> {
        Ok(self.committee.clone())
    }

    /// Returns `false` for all queries.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool> {
        trace!("[MockLedgerService] Contains certificate ID {} - false", fmt_id(certificate_id));
        Ok(false)
    }

    /// Returns `false` for all queries.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool> {
        trace!(
            "[MockLedgerService] Contains transmission ID {}.{} - false",
            fmt_id(transmission_id),
            fmt_id(transmission_id.checksum().unwrap_or_default())
        );
        Ok(false)
    }

    /// Ensures that the given transmission is not a fee and matches the given transmission ID.
    fn ensure_transmission_is_well_formed(
        &self,
        transmission_id: TransmissionID<N>,
        _transmission: &mut Transmission<N>,
    ) -> Result<()> {
        trace!(
            "[MockLedgerService] Ensure transmission ID matches {}.{} - Ok",
            fmt_id(transmission_id),
            fmt_id(transmission_id.checksum().unwrap_or_default())
        );
        Ok(())
    }

    /// Checks the given solution is well-formed.
    async fn check_solution_basic(&self, solution_id: SolutionID<N>, _solution: Data<Solution<N>>) -> Result<()> {
        trace!("[MockLedgerService] Check solution basic {:?} - Ok", fmt_id(solution_id));
        Ok(())
    }

    /// Checks the given transaction is well-formed and unique.
    async fn check_transaction_basic(
        &self,
        transaction_id: N::TransactionID,
        _transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        trace!("[MockLedgerService] Check transaction basic {:?} - Ok", fmt_id(transaction_id));
        Ok(())
    }

    /// Checks the given block is valid next block.
    fn check_next_block(&self, _block: &Block<N>) -> Result<()> {
        Ok(())
    }

    /// Returns a candidate for the next block in the ledger, using a committed subdag and its transmissions.
    #[cfg(feature = "ledger-write")]
    fn prepare_advance_to_next_quorum_block(
        &self,
        _subdag: Subdag<N>,
        _transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<Block<N>> {
        unreachable!("MockLedgerService does not support prepare_advance_to_next_quorum_block")
    }

    /// Adds the given block as the next block in the ledger.
    #[cfg(feature = "ledger-write")]
    fn advance_to_next_block(&self, block: &Block<N>) -> Result<()> {
        ensure!(
            block.height() == self.latest_block_height() + 1,
            "Tried to advance to block {} from block {}",
            block.height(),
            self.latest_block_height()
        );
        self.height_to_round_and_hash.lock().insert(block.height(), (block.round(), block.hash()));
        Ok(())
    }
}
