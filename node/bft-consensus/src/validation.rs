// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use anyhow::{bail, Result};
use bytes::BytesMut;
use narwhal_types::Batch;
use tracing::*;

use snarkos_node_consensus::Consensus as AleoConsensus;
use snarkos_node_messages::Message;
use snarkvm::prelude::{ConsensusStorage, Network};

// An object the BFT consensus workers can use to validate incoming transactions and their batches.
#[derive(Clone)]
pub struct TransactionValidator<N: Network, C: ConsensusStorage<N>>(pub AleoConsensus<N, C>);

impl<N: Network, C: ConsensusStorage<N>> narwhal_worker::TransactionValidator for TransactionValidator<N, C> {
    type Error = anyhow::Error;

    /// Determines if a transaction is valid for the worker to consider putting in a batch
    fn validate(&self, transaction: &[u8]) -> Result<(), Self::Error> {
        let bytes = BytesMut::from(transaction);
        let message = Message::<N>::deserialize(bytes)?;

        let unconfirmed_transaction = if let Message::UnconfirmedTransaction(unconfirmed_transaction) = message {
            unconfirmed_transaction
        } else {
            bail!("[UnconfirmedTransaction] Expected Message::UnconfirmedTransaction, got {:?}", message.name());
        };

        let transaction = match unconfirmed_transaction.transaction.deserialize_blocking() {
            Ok(transaction) => transaction,
            Err(error) => bail!("[UnconfirmedTransaction] {error}"),
        };

        if let Err(err) = self.0.check_transaction_basic(&transaction) {
            error!("Failed to validate a transaction: {err}");
            return Err(err);
        }

        Ok(())
    }

    /// Determines if this batch can be voted on
    fn validate_batch(&self, batch: &Batch) -> Result<(), Self::Error> {
        for transaction in &batch.transactions {
            self.validate(transaction)?;
        }

        Ok(())
    }
}
