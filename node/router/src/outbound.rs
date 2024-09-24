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

use crate::{
    messages::{Message, Ping},
    Router,
};
use snarkos_node_sync_locators::BlockLocators;
use snarkos_node_tcp::protocols::Writing;
use snarkvm::prelude::Network;
use std::io;

use std::net::SocketAddr;
use tokio::sync::oneshot;

pub trait Outbound<N: Network>: Writing<Message = Message<N>> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N>;

    /// Returns `true` if the node is synced up to the latest block (within the given tolerance).
    fn is_block_synced(&self) -> bool;

    /// Returns the number of blocks this node is behind the greatest peer height.
    fn num_blocks_behind(&self) -> u32;

    /// Sends a "Ping" message to the given peer.
    fn send_ping(&self, peer_ip: SocketAddr, block_locators: Option<BlockLocators<N>>) {
        self.send(peer_ip, Message::Ping(Ping::new(self.router().node_type(), block_locators)));
    }

    /// Sends the given message to specified peer.
    ///
    /// This function returns as soon as the message is queued to be sent,
    /// without waiting for the actual delivery; instead, the caller is provided with a [`oneshot::Receiver`]
    /// which can be used to determine when and whether the message has been delivered.
    fn send(&self, peer_ip: SocketAddr, message: Message<N>) -> Option<oneshot::Receiver<io::Result<()>>> {
        // Determine whether to send the message.
        if !self.can_send(peer_ip, &message) {
            return None;
        }
        // Resolve the listener IP to the (ambiguous) peer address.
        let peer_addr = match self.router().resolve_to_ambiguous(&peer_ip) {
            Some(peer_addr) => peer_addr,
            None => {
                warn!("Unable to resolve the listener IP address '{peer_ip}'");
                return None;
            }
        };
        // If the message type is a block request, add it to the cache.
        if let Message::BlockRequest(request) = message {
            self.router().cache.insert_outbound_block_request(peer_ip, request);
        }
        // If the message type is a puzzle request, increment the cache.
        if matches!(message, Message::PuzzleRequest(_)) {
            self.router().cache.increment_outbound_puzzle_requests(peer_ip);
        }
        // If the message type is a peer request, increment the cache.
        if matches!(message, Message::PeerRequest(_)) {
            self.router().cache.increment_outbound_peer_requests(peer_ip);
        }
        // Retrieve the message name.
        let name = message.name();
        // Send the message to the peer.
        trace!("Sending '{name}' to '{peer_ip}'");
        let result = self.unicast(peer_addr, message);
        // If the message was unable to be sent, disconnect.
        if let Err(e) = &result {
            warn!("Failed to send '{name}' to '{peer_ip}': {e}");
            debug!("Disconnecting from '{peer_ip}' (unable to send)");
            self.router().disconnect(peer_ip);
        }
        result.ok()
    }

    /// Sends the given message to every connected peer, excluding the sender and any specified peer IPs.
    fn propagate(&self, message: Message<N>, excluded_peers: &[SocketAddr]) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedSolution(ref mut message) = message {
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
        let connected_peers = self.router().connected_peers();
        let peers = connected_peers.iter().filter(|peer_ip| !excluded_peers.contains(peer_ip));

        // Iterate through all peers that are not the sender and excluded peers.
        for peer_ip in peers {
            self.send(*peer_ip, message.clone());
        }
    }

    /// Sends the given message to every connected validator, excluding the sender and any specified IPs.
    fn propagate_to_validators(&self, message: Message<N>, excluded_peers: &[SocketAddr]) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::UnconfirmedSolution(ref mut message) = message {
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
        let connected_validators = self.router().connected_validators();
        let peers = connected_validators.iter().filter(|peer_ip| !excluded_peers.contains(peer_ip));

        // Iterate through all validators that are not the sender and excluded validators.
        for peer_ip in peers {
            self.send(*peer_ip, message.clone());
        }
    }

    /// Returns `true` if the message can be sent.
    fn can_send(&self, peer_ip: SocketAddr, message: &Message<N>) -> bool {
        // Ensure the peer is connected before sending.
        if !self.router().is_connected(&peer_ip) {
            warn!("Attempted to send to a non-connected peer {peer_ip}");
            return false;
        }
        // Determine whether to send the message.
        match message {
            Message::UnconfirmedSolution(message) => {
                // Update the timestamp for the unconfirmed solution.
                let seen_before = self.router().cache.insert_outbound_solution(peer_ip, message.solution_id).is_some();
                // Determine whether to send the solution.
                !seen_before
            }
            Message::UnconfirmedTransaction(message) => {
                // Update the timestamp for the unconfirmed transaction.
                let seen_before =
                    self.router().cache.insert_outbound_transaction(peer_ip, message.transaction_id).is_some();
                // Determine whether to send the transaction.
                !seen_before
            }
            // For all other message types, return `true`.
            _ => true,
        }
    }
}
