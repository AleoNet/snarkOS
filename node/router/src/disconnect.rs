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

use crate::{Router, Routes};
use snarkos_node_tcp::protocols::Disconnect;
use snarkvm::prelude::Network;

use std::net::SocketAddr;

#[async_trait]
impl<N: Network, R: Routes<N>> Disconnect for Router<N, R> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        // Remove an entry for this `Peer` in the connected peers, if it exists.
        self.connected_peers.write().remove(&peer_addr);
        // Add an entry for this `Peer` in the candidate peers.
        self.candidate_peers.write().insert(peer_addr);
    }
}
