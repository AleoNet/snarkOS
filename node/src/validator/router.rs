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
impl<N: Network> Handshake for Validator<N> {
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 1_000;
}

#[async_trait]
impl<N: Network> Inbound<N> for Validator<N> {
    /// Retrieves the latest epoch challenge and latest block, and returns the puzzle response to the peer.
    async fn puzzle_request(&self, peer_ip: SocketAddr, router: &Router<N>) -> bool {
        // Send the latest puzzle response, if it exists.
        if let Some(puzzle_response) = self.latest_puzzle_response.read().await.clone() {
            // Send the `PuzzleResponse` message to the peer.
            let message = Message::PuzzleResponse(puzzle_response);
            if let Err(error) = router.process(RouterRequest::MessageSend(peer_ip, message)).await {
                warn!("[PuzzleResponse] {error}");
            }
        }
        true
    }

    /// Saves the latest epoch challenge and latest block in the node.
    async fn puzzle_response(&self, message: PuzzleResponse<N>, peer_ip: SocketAddr) -> bool {
        let epoch_challenge = message.epoch_challenge;
        match message.block.deserialize().await {
            Ok(block) => {
                // Retrieve the epoch number.
                let epoch_number = epoch_challenge.epoch_number();
                // Retrieve the block height.
                let block_height = block.height();

                // Save the latest puzzle response in the node.
                self.latest_puzzle_response
                    .write()
                    .await
                    .replace(PuzzleResponse { epoch_challenge, block: Data::Object(block) });

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
impl<N: Network> Outbound for Validator<N> {}
