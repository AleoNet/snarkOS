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

use crate::{external::Block, internal::context::Context};
use snarkos_errors::network::SendError;

use std::{net::SocketAddr, sync::Arc};

/// Broadcast block to connected peers
pub async fn propagate_block(
    context: Arc<Context>,
    block_bytes: Vec<u8>,
    block_miner: SocketAddr,
) -> Result<(), SendError> {
    debug!("Propagating a block to peers");

    let peer_book = context.peer_book.read().await;
    let local_address = *context.local_address.read().await;
    let connections = context.connections.read().await;
    let mut num_peers = 0u16;

    for (socket, _) in peer_book.get_all_connected() {
        if *socket != block_miner && *socket != local_address {
            if let Some(channel) = connections.get(socket) {
                match channel.write(&Block::new(block_bytes.clone())).await {
                    Ok(_) => num_peers += 1,
                    Err(error) => warn!(
                        "Failed to propagate block to peer {}. (error message: {})",
                        channel.address, error
                    ),
                }
            }
        }
    }

    debug!("Block propagated to {} peers", num_peers);

    Ok(())
}
