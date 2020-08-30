// Copyright (C) 2019-2020 Aleo Systems Inc.
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

pub(self) mod connection_handler;

pub(self) mod message_handler;

pub mod miner_instance;
pub use miner_instance::*;

pub mod server;
pub use server::*;

use crate::{
    message_types::{Block, Transaction},
    Context,
};
use snarkos_consensus::{
    memory_pool::{Entry, MemoryPool},
    ConsensusParameters,
    MerkleTreeLedger,
};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_errors::network::SendError;
use snarkos_utilities::bytes::FromBytes;

use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

/// Verify a transaction, add it to the memory pool, propagate it to peers.
pub async fn process_transaction_internal(
    context: Arc<Context>,
    consensus: &ConsensusParameters,
    parameters: &PublicParameters<Components>,
    storage: Arc<MerkleTreeLedger>,
    memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
    transaction_bytes: Vec<u8>,
    transaction_sender: SocketAddr,
) -> Result<(), SendError> {
    if let Ok(transaction) = Tx::read(&transaction_bytes[..]) {
        let mut memory_pool = memory_pool_lock.lock().await;

        if !consensus.verify_transaction(parameters, &transaction, &storage)? {
            error!("Received transaction was invalid");
            return Ok(());
        }

        if transaction.value_balance.is_negative() {
            error!("Received transaction was a coinbase transaction");
            return Ok(());
        }

        let entry = Entry::<Tx> {
            size: transaction_bytes.len(),
            transaction,
        };

        if let Ok(inserted) = memory_pool.insert(&storage, entry) {
            if inserted.is_some() {
                debug!("Transaction added to mempool. Propagating transaction to peers");

                for (socket, _) in &context.peer_book.read().await.get_connected() {
                    if *socket != transaction_sender && *socket != *context.local_address.read().await {
                        if let Some(channel) = context.connections.read().await.get(socket) {
                            channel.write(&Transaction::new(transaction_bytes.clone())).await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Broadcast block to connected peers
pub async fn propagate_block(context: Arc<Context>, data: Vec<u8>, block_miner: SocketAddr) -> Result<(), SendError> {
    debug!("Propagating block to peers");

    for (socket, _) in &context.peer_book.read().await.get_connected() {
        if *socket != block_miner && *socket != *context.local_address.read().await {
            if let Some(channel) = context.connections.read().await.get(socket) {
                channel.write(&Block::new(data.clone())).await?;
            }
        }
    }
    Ok(())
}
