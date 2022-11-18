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

use crate::{Peer, Router};
use snarkos_node_messages::{Data, Message, MessageCodec};
use snarkvm::prelude::Network;

use futures::SinkExt;
use std::time::SystemTime;
use tokio::net::TcpStream;
use tokio_util::codec::Framed;

#[async_trait]
pub trait Outbound {
    /// Handles the sending of a message to a peer.
    async fn outbound<N: Network>(
        &self,
        peer: &Peer<N>,
        mut message: Message<N>,
        router: &Router<N>,
        outbound_socket: &mut Framed<TcpStream, MessageCodec<N>>,
    ) {
        // Retrieve the peer IP.
        let peer_ip = peer.ip();

        // Ensure sufficient time has passed before needing to send the message.
        let is_ready_to_send = match message {
            Message::PuzzleResponse(ref mut message) => {
                // Perform non-blocking serialization of the block (if it hasn't been serialized yet).
                if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
                    let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
                    true
                } else {
                    error!("Puzzle serialization is bugged");
                    false
                }
            }
            Message::UnconfirmedBlock(ref mut message) => {
                let block_height = message.block_height;
                let block_hash = message.block_hash;

                // Update the timestamp for the unconfirmed block.
                let seen_before =
                    router.seen_outbound_blocks.write().await.insert(block_hash, SystemTime::now()).is_some();

                // Determine whether to send the block.
                let mut should_send = !seen_before;

                if should_send {
                    trace!("Preparing 'UnconfirmedBlock {block_height}' for '{peer_ip}'");

                    // Perform non-blocking serialization of the block (if it hasn't been serialized yet).
                    if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
                        let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
                    } else {
                        error!("Block serialization is bugged");
                        should_send = false;
                    }
                }
                should_send
            }
            Message::UnconfirmedSolution(ref mut message) => {
                let puzzle_commitment = message.puzzle_commitment;

                // Update the timestamp for the unconfirmed solution.
                let seen_before =
                    router.seen_outbound_solutions.write().await.insert(puzzle_commitment, SystemTime::now()).is_some();

                // Determine whether to send the solution.
                let mut should_send = !seen_before;

                if should_send {
                    trace!("Preparing 'UnconfirmedSolution' for '{peer_ip}'");

                    // Perform non-blocking serialization of the solution (if it hasn't been serialized yet).
                    if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
                        let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
                    } else {
                        error!("Solution serialization is bugged");
                        should_send = false;
                    }
                }
                should_send
            }
            Message::UnconfirmedTransaction(ref mut message) => {
                let transaction_id = message.transaction_id;

                // Update the timestamp for the unconfirmed transaction.
                let seen_before =
                    router.seen_outbound_transactions.write().await.insert(transaction_id, SystemTime::now()).is_some();

                // Determine whether to send the transaction.
                let mut should_send = !seen_before;

                if should_send {
                    trace!("Preparing 'UnconfirmedTransaction {transaction_id}' for '{peer_ip}'");

                    // Perform non-blocking serialization of the transaction (if it hasn't been serialized yet).
                    if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
                        let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
                    } else {
                        error!("Transaction serialization is bugged");
                        should_send = false;
                    }
                }
                should_send
            }
            _ => true,
        };

        // Send the message, if it is ready.
        if is_ready_to_send {
            trace!("Sending '{}' to '{peer_ip}'", message.name());
            // Route the message to the peer.
            if let Err(error) = outbound_socket.send(message).await {
                warn!("[OutboundRouter] {error}");
            }
        }
    }
}
