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
use snarkos_node_messages::{BlockLocators, Message, Ping};
use snarkos_node_tcp::protocols::Writing;
use snarkvm::prelude::Network;
use std::io;

use std::net::SocketAddr;
use tokio::sync::oneshot;

pub trait Outbound<N: Network>: Writing<Message = Message<N>> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N>;

    /// Sends a "Ping" message to the given peer.
    fn send_ping(&self, peer_ip: SocketAddr, block_locators: Option<BlockLocators<N>>) {
        self.send(
            peer_ip,
            Message::Ping(Ping::<N> {
                version: Message::<N>::VERSION,
                node_type: self.router().node_type(),
                block_locators,
            }),
        );
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
    fn propagate(&self, message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::BeaconPropose(ref mut message) = message {
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

    /// Sends the given message to every connected beacon, excluding the sender and any specified IPs.
    fn propagate_to_beacons(&self, message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::BeaconPropose(ref mut message) = message {
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
            .filter(|peer_ip| !self.router().is_local_ip(peer_ip) && !excluded_peers.contains(peer_ip))
            .copied()
            .collect::<Vec<_>>();

        // Iterate through all beacons that are not the sender and excluded beacons.
        for peer_ip in peers {
            self.send(peer_ip, message.clone());
        }
    }

    /// Sends the given message to every connected validator, excluding the sender and any specified IPs.
    fn propagate_to_validators(&self, message: Message<N>, excluded_peers: Vec<SocketAddr>) {
        // TODO (howardwu): Serialize large messages once only.
        // // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        // if let Message::BeaconPropose(ref mut message) = message {
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
            .connected_validators()
            .iter()
            .filter(|peer_ip| !self.router().is_local_ip(peer_ip) && !excluded_peers.contains(peer_ip))
            .copied()
            .collect::<Vec<_>>();

        // Iterate through all beacons that are not the sender and excluded beacons.
        for peer_ip in peers {
            self.send(peer_ip, message.clone());
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
                let seen_before =
                    self.router().cache.insert_outbound_solution(peer_ip, message.puzzle_commitment).is_some();
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
