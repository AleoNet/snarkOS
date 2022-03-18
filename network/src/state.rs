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

use crate::{ledger::Ledger, operator::Operator, peers::Peers, prover::Prover};

use snarkos_environment::Environment;
use snarkvm::prelude::*;

use std::{net::SocketAddr, sync::Arc};

/// The network state of the node.
#[derive(Debug, Clone)]
pub struct NetworkState<N: Network, E: Environment> {
    /// The local address of the node.
    pub local_ip: SocketAddr,
    /// The list of peers for the node.
    pub peers: Arc<Peers<N, E>>,
    /// The ledger of the node.
    pub ledger: Arc<Ledger<N, E>>,
    /// The operator of the node.
    pub operator: Arc<Operator<N, E>>,
    /// The prover of the node.
    pub prover: Arc<Prover<N, E>>,
}

impl<N: Network, E: Environment> NetworkState<N, E> {
    pub fn new(
        local_ip: SocketAddr,
        peers: Arc<Peers<N, E>>,
        ledger: Arc<Ledger<N, E>>,
        operator: Arc<Operator<N, E>>,
        prover: Arc<Prover<N, E>>,
    ) -> Self {
        let network_state = Self {
            local_ip,
            peers,
            ledger,
            operator,
            prover,
        };

        // Set the network state reference on the various services.
        network_state.peers.set_network_state(network_state.clone());
        network_state.ledger.set_network_state(network_state.clone());
        network_state.operator.set_network_state(network_state.clone());
        network_state.prover.set_network_state(network_state.clone());

        network_state
    }
}
