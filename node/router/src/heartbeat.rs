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

use crate::{Outbound, Router};
use snarkos_node_messages::{DisconnectReason, Message, PeerRequest};
use snarkvm::prelude::Network;

use rand::{prelude::IteratorRandom, rngs::OsRng};

#[async_trait]
pub trait Heartbeat<N: Network>: Outbound<N> {
    /// The duration in seconds to sleep in between heartbeat executions.
    const HEARTBEAT_IN_SECS: u64 = 9; // 9 seconds
    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize = 2;
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;

    /// Handles the heartbeat request.
    async fn heartbeat(&self) {
        assert!(Self::MINIMUM_NUMBER_OF_PEERS <= Self::MAXIMUM_NUMBER_OF_PEERS);

        // Log the connected peers.
        let connected_peers = self.router().connected_peers();
        match connected_peers.len() {
            0 => debug!("No connected peers"),
            1 => debug!("Connected to 1 peer: {connected_peers:?}"),
            num_connected => debug!("Connected to {num_connected} peers: {connected_peers:?}"),
        }

        // Remove the oldest connected peer.
        self.remove_oldest_connected_peer().await;
        // Remove any stale connected peers.
        self.remove_stale_connected_peers().await;
        // Keep the number of connected beacons within the allowed range.
        self.handle_connected_beacons().await;
        // Keep the number of connected peers within the allowed range.
        self.handle_connected_peers().await;
        // Keep the bootstrap peers within the allowed range.
        self.handle_bootstrap_peers().await;
        // Keep the trusted peers connected.
        self.handle_trusted_peers().await;
    }

    /// This function removes the oldest connected peer, to keep the connections fresh.
    /// This function only triggers if the router is above the minimum number of connected peers.
    async fn remove_oldest_connected_peer(&self) {
        // Check if the router is above the minimum number of connected peers.
        if self.router().number_of_connected_peers() > Self::MINIMUM_NUMBER_OF_PEERS {
            // Disconnect from the oldest connected peer, if one exists.
            if let Some(oldest) = self.router().oldest_connected_peer() {
                info!("Disconnecting from '{oldest}' (periodic refresh of peers)");
                self.send(oldest, Message::Disconnect(DisconnectReason::PeerRefresh.into()));
                // Disconnect from this peer.
                self.router().disconnect(oldest).await;
            }
        }
    }

    /// This function removes any connected peers that have not communicated within the predefined time.
    async fn remove_stale_connected_peers(&self) {
        // Check if any connected peer is stale.
        for peer in self.router().connected_peers_inner().into_values() {
            // Disconnect if the peer has not communicated back within the predefined time.
            let elapsed = peer.last_seen().elapsed().as_secs();
            if elapsed > Router::<N>::RADIO_SILENCE_IN_SECS {
                warn!("Peer {} has not communicated in {elapsed} seconds", peer.ip());
                // Disconnect from this peer.
                self.router().disconnect(peer.ip()).await;
            }
        }
    }

    /// This function keeps the number of connected beacons within the allowed range.
    async fn handle_connected_beacons(&self) {
        // Determine if the node is connected to more beacons than allowed.
        let connected_beacons = self.router().connected_beacons();
        let num_surplus = connected_beacons.len().saturating_sub(1);
        if num_surplus > 0 {
            // Initialize an RNG.
            let rng = &mut OsRng::default();
            // Proceed to send disconnect requests to these beacons.
            for peer_ip in connected_beacons.into_iter().choose_multiple(rng, num_surplus) {
                info!("Disconnecting from 'beacon' {peer_ip} (exceeded maximum beacons)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip).await;
            }
        }
    }

    /// This function keeps the number of connected peers within the allowed range.
    async fn handle_connected_peers(&self) {
        // Obtain the number of connected peers.
        let num_connected = self.router().number_of_connected_peers();
        // Compute the number of surplus peers.
        let num_surplus = num_connected.saturating_sub(Self::MAXIMUM_NUMBER_OF_PEERS);
        // Compute the number of deficit peers.
        let num_deficient = Self::MINIMUM_NUMBER_OF_PEERS.saturating_sub(num_connected);

        if num_surplus > 0 {
            debug!("Exceeded maximum number of connected peers, disconnecting from {num_surplus} peers");

            // Initialize an RNG.
            let rng = &mut OsRng::default();

            // Determine the peers to disconnect from.
            let peer_ips_to_disconnect = self
                .router()
                .connected_peers()
                .into_iter()
                .filter(|peer_ip| !self.router().trusted_peers().contains(peer_ip))
                .choose_multiple(rng, num_surplus);

            // Proceed to send disconnect requests to these peers.
            for peer_ip in peer_ips_to_disconnect {
                info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip).await;
            }
        }

        if num_deficient > 0 {
            // Initialize an RNG.
            let rng = &mut OsRng::default();

            // Attempt to connect to more peers.
            for peer_ip in self.router().candidate_peers().into_iter().choose_multiple(rng, num_deficient) {
                self.router().connect(peer_ip).await;
            }
            // Request more peers from the connected peers.
            for peer_ip in self.router().connected_peers().into_iter().choose_multiple(rng, 3) {
                self.send(peer_ip, Message::PeerRequest(PeerRequest));
            }
        }
    }

    /// This function keeps the number of bootstrap peers within the allowed range.
    async fn handle_bootstrap_peers(&self) {
        // TODO (howardwu): Remove this for Phase 3.
        if self.router().node_type().is_beacon() {
            return;
        }
        // Find the connected bootstrap peers.
        let connected_bootstrap = self.router().connected_bootstrap_peers();
        // If there are not enough connected bootstrap peers, connect to more.
        if connected_bootstrap.is_empty() {
            // Initialize an RNG.
            let rng = &mut OsRng::default();
            // Attempt to connect to a bootstrap peer.
            if let Some(peer_ip) = self.router().bootstrap_peers().into_iter().choose(rng) {
                self.router().connect(peer_ip).await;
            }
        }

        // Determine if the node is connected to more beacons than allowed.
        let num_surplus = connected_bootstrap.len().saturating_sub(1);
        if num_surplus > 0 {
            // Initialize an RNG.
            let rng = &mut OsRng::default();
            // Proceed to send disconnect requests to these beacons.
            for peer_ip in connected_bootstrap.into_iter().choose_multiple(rng, num_surplus) {
                info!("Disconnecting from '{peer_ip}' (exceeded maximum bootstrap)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip).await;
            }
        }
    }

    /// This function attempts to connect to any disconnected trusted peers.
    async fn handle_trusted_peers(&self) {
        // Ensure that the trusted nodes are connected.
        for peer_ip in self.router().trusted_peers() {
            // If the peer is not connected, attempt to connect to it.
            if !self.router().is_connected(peer_ip) {
                // Attempt to connect to the trusted peer.
                self.router().connect(*peer_ip).await;
            }
        }
    }
}
