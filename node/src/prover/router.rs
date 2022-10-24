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
impl<N: Network> Handshake for Prover<N> {}

#[async_trait]
impl<N: Network> Inbound<N> for Prover<N> {
    /// Saves the latest epoch challenge and latest block in the prover.
    async fn puzzle_response(&self, message: PuzzleResponse<N>) -> bool {
        let epoch_challenge = message.epoch_challenge;
        match message.block.deserialize().await {
            Ok(block) => {
                // Save the latest epoch challenge in the prover.
                self.latest_epoch_challenge.write().await.replace(epoch_challenge);
                // Save the latest block in the prover.
                self.latest_block.write().await.replace(block);
                true
            }
            Err(error) => {
                error!("Failed to deserialize the block from the puzzle response: {error}");
                false
            }
        }
    }
}

#[async_trait]
impl<N: Network> Outbound for Prover<N> {}
