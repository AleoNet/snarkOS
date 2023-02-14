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

use anyhow::{bail, Result};
use bytes::BytesMut;
use narwhal_types::{Batch, BatchAPI};
use tracing::*;

use snarkos_node_consensus::Consensus as AleoConsensus;
use snarkos_node_messages::Message;
use snarkvm::prelude::{ConsensusStorage, Network};

// An object the BFT consensus workers can use to validate incoming transactions and their batches.
#[derive(Clone)]
pub struct TransactionValidator<N: Network, C: ConsensusStorage<N>>(pub AleoConsensus<N, C>);

#[async_trait::async_trait]
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

        if let Err(err) = self.0.check_transaction_basic(&transaction, None) {
            error!("Failed to validate a transaction: {err}");
            return Err(err);
        }

        Ok(())
    }

    /// Determines if this batch can be voted on
    async fn validate_batch(&self, batch: &Batch) -> Result<(), Self::Error> {
        for transaction in batch.transactions() {
            self.validate(transaction)?;
        }

        Ok(())
    }
}
