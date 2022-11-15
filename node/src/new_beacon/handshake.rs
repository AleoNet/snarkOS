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

use crate::new_beacon::Beacon;
use snarkos_node_network::{protocols::Handshake as Handshaking, Connection, ConnectionSide};
use snarkvm::prelude::Network as CurrentNetwork;

use std::io;

#[async_trait::async_trait]
impl<N: CurrentNetwork> Handshaking for Beacon<N> {
    async fn perform_handshake(&self, conn: Connection) -> io::Result<Connection> {
        let peer_side = conn.side();

        match peer_side {
            // The peer initiated the connection.
            ConnectionSide::Initiator => {}

            // The relay initiated the connection.
            ConnectionSide::Responder => {}
        }

        Ok(conn)
    }
}
