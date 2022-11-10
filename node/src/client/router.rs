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

use super::*;

#[async_trait]
impl<N: Network> Handshake for Client<N> {}

#[async_trait]
impl<N: Network> Inbound<N> for Client<N> {
    /// Saves the latest epoch challenge and latest block in the node.
    async fn puzzle_response(&self, message: PuzzleResponse<N>, peer_ip: SocketAddr) -> bool {
        let epoch_challenge = message.epoch_challenge;
        match message.block.deserialize().await {
            Ok(block) => {
                // Retrieve the epoch number.
                let epoch_number = epoch_challenge.epoch_number();
                // Retrieve the block height.
                let block_height = block.height();

                info!(
                    "Current(Epoch {epoch_number}, Block {block_height}, Coinbase Target {}, Proof Target {})",
                    block.coinbase_target(),
                    block.proof_target()
                );

                // Save the latest epoch challenge in the node.
                self.latest_epoch_challenge.write().await.replace(epoch_challenge);
                // Save the latest block in the node.
                self.latest_block.write().await.replace(block);

                trace!("Received 'PuzzleResponse' from '{peer_ip}' (Epoch {epoch_number}, Block {block_height})");
                true
            }
            Err(error) => {
                error!("Failed to deserialize the puzzle response from '{peer_ip}': {error}");
                false
            }
        }
    }
}

#[async_trait]
impl<N: Network> Outbound for Client<N> {}
