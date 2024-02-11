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

use crate::{
    messages::{DisconnectReason, Message, PeerRequest},
    Outbound,
    Router,
};
use snarkvm::prelude::Network;

use colored::Colorize;
use rand::{prelude::IteratorRandom, rngs::OsRng};

/// A helper function to compute the maximum of two numbers.
/// See Rust issue 92391: https://github.com/rust-lang/rust/issues/92391.
pub const fn max(a: usize, b: usize) -> usize {
    match a > b {
        true => a,
        false => b,
    }
}

pub trait Heartbeat<N: Network>: Outbound<N> {
    /// The duration in seconds to sleep in between heartbeat executions.
    const HEARTBEAT_IN_SECS: u64 = 25; // 25 seconds
    /// The minimum number of peers required to maintain connections with.
    const MINIMUM_NUMBER_OF_PEERS: usize = 3;
    /// The median number of peers to maintain connections with.
    const MEDIAN_NUMBER_OF_PEERS: usize = max(Self::MAXIMUM_NUMBER_OF_PEERS / 2, Self::MINIMUM_NUMBER_OF_PEERS);
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 21;

    /// Handles the heartbeat request.
    fn heartbeat(&self) {
        self.safety_check_minimum_number_of_peers();
        self.log_connected_peers();

        // Remove any stale connected peers.
        self.remove_stale_connected_peers();
        // Remove the oldest connected peer.
        self.remove_oldest_connected_peer();
        // Keep the number of connected peers within the allowed range.
        self.handle_connected_peers();
        // Keep the bootstrap peers within the allowed range.
        self.handle_bootstrap_peers();
        // Keep the trusted peers connected.
        self.handle_trusted_peers();
        // Keep the puzzle request up to date.
        self.handle_puzzle_request();
    }

    /// TODO (howardwu): Consider checking minimum number of validators, to exclude clients and provers.
    /// This function performs safety checks on the setting for the minimum number of peers.
    fn safety_check_minimum_number_of_peers(&self) {
        // Perform basic sanity checks on the configuration for the number of peers.
        assert!(Self::MINIMUM_NUMBER_OF_PEERS >= 1, "The minimum number of peers must be at least 1.");
        assert!(Self::MINIMUM_NUMBER_OF_PEERS <= Self::MAXIMUM_NUMBER_OF_PEERS);
        assert!(Self::MINIMUM_NUMBER_OF_PEERS <= Self::MEDIAN_NUMBER_OF_PEERS);
        assert!(Self::MEDIAN_NUMBER_OF_PEERS <= Self::MAXIMUM_NUMBER_OF_PEERS);
    }

    /// This function logs the connected peers.
    fn log_connected_peers(&self) {
        // Log the connected peers.
        let connected_peers = self.router().connected_peers();
        let connected_peers_fmt = format!("{connected_peers:?}").dimmed();
        match connected_peers.len() {
            0 => debug!("No connected peers"),
            1 => debug!("Connected to 1 peer: {connected_peers_fmt}"),
            num_connected => debug!("Connected to {num_connected} peers {connected_peers_fmt}"),
        }
    }

    /// This function removes any connected peers that have not communicated within the predefined time.
    fn remove_stale_connected_peers(&self) {
        // Check if any connected peer is stale.
        for peer in self.router().get_connected_peers() {
            // Disconnect if the peer has not communicated back within the predefined time.
            let elapsed = peer.last_seen().elapsed().as_secs();
            if elapsed > Router::<N>::RADIO_SILENCE_IN_SECS {
                warn!("Peer {} has not communicated in {elapsed} seconds", peer.ip());
                // Disconnect from this peer.
                self.router().disconnect(peer.ip());
            }
        }
    }

    /// This function removes the oldest connected peer, to keep the connections fresh.
    /// This function only triggers if the router is above the minimum number of connected peers.
    fn remove_oldest_connected_peer(&self) {
        // Skip if the router is at or below the minimum number of connected peers.
        if self.router().number_of_connected_peers() <= Self::MINIMUM_NUMBER_OF_PEERS {
            return;
        }

        // Retrieve the trusted peers.
        let trusted = self.router().trusted_peers();
        // Retrieve the bootstrap peers.
        let bootstrap = self.router().bootstrap_peers();

        // Find the oldest connected peer, that is neither trusted nor a bootstrap peer.
        let oldest_peer = self
            .router()
            .get_connected_peers()
            .iter()
            .filter(|peer| !trusted.contains(&peer.ip()) && !bootstrap.contains(&peer.ip()))
            .min_by_key(|peer| peer.last_seen())
            .map(|peer| peer.ip());

        // Disconnect from the oldest connected peer, if one exists.
        if let Some(oldest) = oldest_peer {
            info!("Disconnecting from '{oldest}' (periodic refresh of peers)");
            let _ = self.send(oldest, Message::Disconnect(DisconnectReason::PeerRefresh.into()));
            // Disconnect from this peer.
            self.router().disconnect(oldest);
        }
    }

    /// TODO (howardwu): If the node is a validator, keep the validator.
    /// This function keeps the number of connected peers within the allowed range.
    fn handle_connected_peers(&self) {
        // Obtain the number of connected peers.
        let num_connected = self.router().number_of_connected_peers();
        // Compute the number of surplus peers.
        let num_surplus = num_connected.saturating_sub(Self::MAXIMUM_NUMBER_OF_PEERS);
        // Compute the number of deficit peers.
        let num_deficient = Self::MEDIAN_NUMBER_OF_PEERS.saturating_sub(num_connected);

        if num_surplus > 0 {
            debug!("Exceeded maximum number of connected peers, disconnecting from {num_surplus} peers");

            // Retrieve the trusted peers.
            let trusted = self.router().trusted_peers();
            // Retrieve the bootstrap peers.
            let bootstrap = self.router().bootstrap_peers();

            // Initialize an RNG.
            let rng = &mut OsRng;

            // TODO (howardwu): As a validator, prioritize disconnecting from clients and provers.
            //  Remove RNG, pick the `n` oldest nodes.
            // Determine the peers to disconnect from.
            let peer_ips_to_disconnect = self
                .router()
                .connected_peers()
                .into_iter()
                .filter(|peer_ip| !trusted.contains(peer_ip) && !bootstrap.contains(peer_ip))
                .choose_multiple(rng, num_surplus);

            // Proceed to send disconnect requests to these peers.
            for peer_ip in peer_ips_to_disconnect {
                // TODO (howardwu): Remove this after specializing this function.
                if self.router().node_type().is_prover() {
                    if let Some(peer) = self.router().get_connected_peer(&peer_ip) {
                        if peer.node_type().is_validator() {
                            continue;
                        }
                    }
                }

                info!("Disconnecting from '{peer_ip}' (exceeded maximum connections)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip);
            }
        }

        if num_deficient > 0 {
            // Initialize an RNG.
            let rng = &mut OsRng;

            // Attempt to connect to more peers.
            for peer_ip in self.router().candidate_peers().into_iter().choose_multiple(rng, num_deficient) {
                self.router().connect(peer_ip);
            }
            // Request more peers from the connected peers.
            for peer_ip in self.router().connected_peers().into_iter().choose_multiple(rng, 3) {
                self.send(peer_ip, Message::PeerRequest(PeerRequest));
            }
        }
    }

    /// This function keeps the number of bootstrap peers within the allowed range.
    fn handle_bootstrap_peers(&self) {
        // Split the bootstrap peers into connected and candidate lists.
        let mut connected_bootstrap = Vec::new();
        let mut candidate_bootstrap = Vec::new();
        for bootstrap_ip in self.router().bootstrap_peers() {
            match self.router().is_connected(&bootstrap_ip) {
                true => connected_bootstrap.push(bootstrap_ip),
                false => candidate_bootstrap.push(bootstrap_ip),
            }
        }
        // If there are not enough connected bootstrap peers, connect to more.
        if connected_bootstrap.is_empty() {
            // Initialize an RNG.
            let rng = &mut OsRng;
            // Attempt to connect to a bootstrap peer.
            if let Some(peer_ip) = candidate_bootstrap.into_iter().choose(rng) {
                self.router().connect(peer_ip);
            }
        }
        // Determine if the node is connected to more bootstrap peers than allowed.
        let num_surplus = connected_bootstrap.len().saturating_sub(1);
        if num_surplus > 0 {
            // Initialize an RNG.
            let rng = &mut OsRng;
            // Proceed to send disconnect requests to these bootstrap peers.
            for peer_ip in connected_bootstrap.into_iter().choose_multiple(rng, num_surplus) {
                info!("Disconnecting from '{peer_ip}' (exceeded maximum bootstrap)");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::TooManyPeers.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip);
            }
        }
    }

    /// This function attempts to connect to any disconnected trusted peers.
    fn handle_trusted_peers(&self) {
        // Ensure that the trusted nodes are connected.
        for peer_ip in self.router().trusted_peers() {
            // If the peer is not connected, attempt to connect to it.
            if !self.router().is_connected(peer_ip) {
                // Attempt to connect to the trusted peer.
                self.router().connect(*peer_ip);
            }
        }
    }

    /// This function updates the coinbase puzzle if network has updated.
    fn handle_puzzle_request(&self) {
        // No-op
    }
}
