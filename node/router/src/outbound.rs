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

use crate::Router;
use snarkos_node_messages::{Message, PuzzleRequest};
use snarkos_node_tcp::protocols::Writing;
use snarkvm::prelude::Network;
use std::io;

use std::net::SocketAddr;
use tokio::sync::oneshot;

pub trait Outbound<N: Network>: Writing<Message = Message<N>> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N>;

    /// Sends a "PuzzleRequest" to a bootstrap peer.
    fn send_puzzle_request(&self) {
        // TODO (howardwu): Change this logic for Phase 3.
        // Retrieve a bootstrap peer.
        let bootstrap_ip = match self.router().node_type().is_validator() {
            true => self.router().connected_beacons().first().copied(),
            false => self.router().connected_bootstrap_peers().first().copied(),
        };
        // If a bootstrap peer exists, send a "PuzzleRequest" to it.
        if let Some(bootstrap_ip) = bootstrap_ip {
            // Send the "PuzzleRequest" to the bootstrap peer.
            self.send(bootstrap_ip, Message::PuzzleRequest(PuzzleRequest));
        }
    }

    /// Sends the given message to specified peer.
    ///
    /// This function returns as soon as the message is queued to be sent,
    /// without waiting for the actual delivery; instead, the caller is provided with a [`oneshot::Receiver`]
    /// which can be used to determine when and whether the message has been delivered.
    fn send(&self, peer_ip: SocketAddr, message: Message<N>) -> Option<oneshot::Receiver<io::Result<()>>> {
        // Determine whether to send the message.
        if !self.should_send(peer_ip, &message) {
            return None;
        }
        // Ensure the peer is connected before sending.
        if !self.router().is_connected(&peer_ip) {
            warn!("Attempted to send to a non-connected peer {peer_ip}");
            return None;
        }
        // Retrieve the message name.
        let name = message.name().to_string();
        // Resolve the listener IP to the (ambiguous) peer address.
        if let Some(peer_addr) = self.router().resolve_to_ambiguous(&peer_ip) {
            // Send the message to the peer.
            trace!("Sending '{name}' to '{peer_ip}'");
            self.unicast(peer_addr, message).map_err(|e| warn!("Failed to send '{name}' to '{peer_ip}': {e}")).ok()
        } else {
            warn!("Unable to resolve the listener IP address '{peer_ip}'");
            None
        }
    }

    /// Sends the given message to every connected peer, excluding the sender and any specified peer IPs.
    fn propagate(&self, message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedBlock(ref mut message) = message {
        //     if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
        //         let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
        //     } else {
        //         error!("Block serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedSolution(ref mut message) = message {
        //     if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
        //         let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
        //     } else {
        //         error!("Solution serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedTransaction(ref mut message) = message {
        //     if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
        //         let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
        //     } else {
        //         error!("Transaction serialization is bugged");
        //     }
        // }

        // Prepare the peers to send to.
        let peers = self
            .router()
            .connected_peers()
            .iter()
            .filter(|peer_ip| !self.router().is_local_ip(peer_ip) && !excluded_peers.contains(peer_ip))
            .copied()
            .collect::<Vec<_>>();

        // Iterate through all peers that are not the sender and excluded peers.
        for peer_ip in peers {
            self.send(peer_ip, message.clone());
        }
    }

    /// Sends the given message to every connected beacon, excluding the sender and any specified beacon IPs.
    fn propagate_to_beacons(&self, message: Message<N>, excluded_beacons: Vec<SocketAddr>) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedBlock(ref mut message) = message {
        //     if let Ok(serialized_block) = Data::serialize(message.block.clone()).await {
        //         let _ = std::mem::replace(&mut message.block, Data::Buffer(serialized_block));
        //     } else {
        //         error!("Block serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedSolution(ref mut message) = message {
        //     if let Ok(serialized_solution) = Data::serialize(message.solution.clone()).await {
        //         let _ = std::mem::replace(&mut message.solution, Data::Buffer(serialized_solution));
        //     } else {
        //         error!("Solution serialization is bugged");
        //     }
        // } else if let Message::UnconfirmedTransaction(ref mut message) = message {
        //     if let Ok(serialized_transaction) = Data::serialize(message.transaction.clone()).await {
        //         let _ = std::mem::replace(&mut message.transaction, Data::Buffer(serialized_transaction));
        //     } else {
        //         error!("Transaction serialization is bugged");
        //     }
        // }

        // Prepare the peers to send to.
        let peers = self
            .router()
            .connected_beacons()
            .iter()
            .filter(|peer_ip| !self.router().is_local_ip(peer_ip) && !excluded_beacons.contains(peer_ip))
            .copied()
            .collect::<Vec<_>>();

        // Iterate through all beacons that are not the sender and excluded beacons.
        for peer_ip in peers {
            self.send(peer_ip, message.clone());
        }
    }

    /// Returns `true` if the message should be sent.
    fn should_send(&self, peer_ip: SocketAddr, message: &Message<N>) -> bool {
        // Determine whether to send the message.
        let should_send = match message {
            Message::UnconfirmedBlock(message) => {
                // Update the timestamp for the unconfirmed block.
                let seen_before = self.router().cache.insert_outbound_block(message.block_hash).is_some();
                // Determine whether to send the block.
                !seen_before
            }
            Message::UnconfirmedSolution(message) => {
                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.router().cache.insert_outbound_solution(message.puzzle_commitment).is_some();
                // Determine whether to send the solution.
                !seen_before
            }
            Message::UnconfirmedTransaction(message) => {
                // Update the timestamp for the unconfirmed transaction.
                let seen_before = self.router().cache.insert_outbound_transaction(message.transaction_id).is_some();
                // Determine whether to send the transaction.
                !seen_before
            }
            // For all other message types, return `true`.
            _ => true,
        };
        // If the message should be sent and the message type is a puzzle request, increment the cache.
        if should_send && matches!(message, Message::PuzzleRequest(_)) {
            self.router().cache.increment_outbound_puzzle_requests(peer_ip);
        }
        // Return whether the message should be sent.
        should_send
    }
}
