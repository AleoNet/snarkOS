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
use snarkvm::prelude::{ProverSolution, PuzzleCommitment};

#[async_trait]
impl<N: Network> Handshake for Beacon<N> {}

#[async_trait]
impl<N: Network> Inbound<N> for Beacon<N> {
    /// Retrieves the latest epoch challenge and latest block, and returns the puzzle response to the peer.
    async fn puzzle_request(&self, peer_ip: SocketAddr, router: &Router<N>) -> bool {
        // Retrieve the latest epoch challenge and latest block.
        let (epoch_challenge, block) = {
            // Acquire a read lock on the consensus module.
            let consensus = self.ledger.consensus().read();

            // Retrieve the latest epoch challenge.
            let epoch_challenge = match consensus.latest_epoch_challenge() {
                Ok(block) => block,
                Err(error) => {
                    error!("Failed to retrieve latest epoch challenge for a puzzle request: {error}");
                    return false;
                }
            };

            // Retrieve the latest block.
            let block = match consensus.latest_block() {
                Ok(block) => block,
                Err(error) => {
                    error!("Failed to retrieve latest block for a puzzle request: {error}");
                    return false;
                }
            };

            // Scope drops the read lock on the consensus module.
            (epoch_challenge, block)
        };
        // Send the `PuzzleResponse` message to the peer.
        let message = Message::PuzzleResponse(PuzzleResponse { epoch_challenge, block: Data::Object(block) });
        if let Err(error) = router.process(RouterRequest::MessageSend(peer_ip, message)).await {
            warn!("[PuzzleResponse] {}", error);
        }

        true
    }

    /// Adds the unconfirmed solution to the memory pool, and propagates the solution to all peers.
    async fn unconfirmed_solution(
        &self,
        message: UnconfirmedSolution<N>,
        _puzzle_commitment: PuzzleCommitment<N>,
        solution: ProverSolution<N>,
        peer_ip: SocketAddr,
        router: &Router<N>,
        seen_before: bool,
    ) -> bool {
        // Determine whether to propagate the solution.
        let should_propagate = !seen_before;

        if !should_propagate {
            trace!("Skipping 'UnconfirmedSolution' from '{peer_ip}'");
        } else {
            // Add the unconfirmed solution to the memory pool.
            if let Err(error) = self.ledger.consensus().write().add_unconfirmed_solution(&solution) {
                trace!("[UnconfirmedSolution] {error}");
                return true; // Maintain the connection.
            }

            // Propagate the `UnconfirmedSolution`.
            let request = RouterRequest::MessagePropagate(Message::UnconfirmedSolution(message), vec![peer_ip]);
            if let Err(error) = router.process(request).await {
                warn!("[UnconfirmedSolution] {error}");
            }
        }
        true
    }
}

#[async_trait]
impl<N: Network> Outbound for Beacon<N> {}
