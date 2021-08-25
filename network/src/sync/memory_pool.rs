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

use crate::{message::*, NetworkError, Node};
use snarkos_storage::VMTransaction;
use snarkvm_dpc::testnet1::instantiated::Testnet1Transaction;
use snarkvm_utilities::bytes::{FromBytes, ToBytes};

use anyhow::*;
use std::{net::SocketAddr, time::Instant};

impl Node {
    ///
    /// Triggers the memory pool sync with a selected peer.
    ///
    pub async fn update_memory_pool(&self, sync_node: Option<SocketAddr>) {
        if let Some(sync_node) = sync_node {
            info!("Updating memory pool from {}", sync_node);

            self.peer_book.send_to(sync_node, Payload::GetMemoryPool, None).await;
        } else {
            debug!("No sync node is registered, memory pool could not be synced");
        }
    }

    ///
    /// Broadcast memory pool transaction to connected peers.
    ///
    pub(crate) async fn propagate_memory_pool_transaction(
        &self,
        transaction_bytes: Vec<u8>,
        transaction_sender: SocketAddr,
    ) {
        debug!("Propagating a memory pool transaction to connected peers");

        let local_address = self.local_address().unwrap();

        for remote_address in self.connected_peers() {
            if remote_address != transaction_sender && remote_address != local_address {
                // Send a `Transaction` message to the connected peer.
                self.peer_book
                    .send_to(remote_address, Payload::Transaction(transaction_bytes.clone()), None)
                    .await;
            }
        }
    }

    ///
    /// Verifies a received memory pool transaction, adds it to the memory pool,
    /// and propagates it to peers.
    ///
    pub(crate) async fn received_memory_pool_transaction(
        &self,
        source: SocketAddr,
        transaction: Vec<u8>,
    ) -> Result<()> {
        let tx = Testnet1Transaction::read_le(&*transaction)?;

        let inserted = self.expect_sync().consensus.receive_transaction(tx.serialize()?).await;

        if inserted {
            info!("Transaction added to memory pool.");
            self.propagate_memory_pool_transaction(transaction, source).await;
        }

        Ok(())
    }

    /// A peer has requested our memory pool transactions.
    pub(crate) async fn received_get_memory_pool(
        &self,
        remote_address: SocketAddr,
        time_received: Option<Instant>,
    ) -> Result<()> {
        let transactions = self
            .expect_sync()
            .consensus
            .fetch_memory_pool()
            .await
            .into_iter()
            .map(|tx| {
                let mut out = vec![];
                tx.write_le(&mut out)?;
                Ok(out)
            })
            .collect::<Result<Vec<Vec<u8>>>>()?;

        if !transactions.is_empty() {
            // Send a `MemoryPool` message to the connected peer.
            self.peer_book
                .send_to(remote_address, Payload::MemoryPool(transactions), time_received)
                .await;
        }
        Ok(())
    }

    /// A peer has sent us their memory pool transactions.
    pub(crate) async fn received_memory_pool(&self, transactions: Vec<Vec<u8>>) -> Result<(), NetworkError> {
        // todo: if the txn counts here are large, batch into storage
        for transaction_bytes in transactions {
            let transaction = Testnet1Transaction::read_le(&transaction_bytes[..])?;
            let inserted = self
                .expect_sync()
                .consensus
                .receive_transaction(transaction.serialize()?)
                .await;

            if inserted {
                info!("Transaction added to memory pool from batch.");
            }
        }

        Ok(())
    }
}
