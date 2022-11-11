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
    /// Retrieves the blocks within the block request range, and returns the block response to the peer.
    async fn block_request(&self, message: BlockRequest, peer_ip: SocketAddr, router: &Router<N>) -> bool {
        // Ensure the block request is well formed.
        if message.start_block_height > message.end_block_height {
            debug!("Invalid BlockRequest received from '{peer_ip}' - start height is greater than end height");
            return false;
        }

        // Ensure that the block request is within the proper bounds.
        if message.end_block_height - message.start_block_height > Self::MAXIMUM_BLOCK_REQUEST {
            debug!("Invalid BlockRequest received from '{peer_ip}' - exceeds maximum block request");
            return false;
        }

        // Retrieve the requested blocks.
        let blocks = match self.ledger.get_blocks(message.start_block_height, message.end_block_height) {
            Ok(blocks) => blocks,
            Err(error) => {
                error!(
                    "Failed to retrieve blocks {} to {} from the ledger: {error}",
                    message.start_block_height, message.end_block_height
                );
                return false;
            }
        };

        // Send the `BlockResponse` message to the peer.
        let message = Message::BlockResponse(BlockResponse::new(blocks));
        if let Err(error) = router.process(RouterRequest::MessageSend(peer_ip, message)).await {
            warn!("[BlockResponse] {}", error);
        }

        true
    }

    async fn block_response(&self, message: BlockResponse<N>, peer_ip: SocketAddr, _router: &Router<N>) -> bool {
        // Deserialize the block response.
        let blocks = match message.blocks.deserialize().await {
            Ok(blocks) => blocks,
            Err(error) => {
                error!("Failed to deserialize blocks from '{peer_ip}': {error}");
                return false;
            }
        };

        // Add the blocks from the block response to the ledger.
        for block in blocks.0 {
            // Check that the next block is valid.
            if let Err(error) = self.consensus.check_next_block(&block) {
                trace!("[BlockResponse] {error}");
                return true;
            }

            // Attempt to add the block to the ledger.
            if let Err(error) = self.consensus.advance_to_next_block(&block) {
                trace!("[BlockResponse] {error}");
                return true;
            }

            info!("Ledger successfully advanced to block {} ({})", block.height(), block.hash());
        }

        true
    }

    /// Send a ping message to the peer after `PING_SLEEP_IN_SECS` seconds.
    async fn pong(&self, _message: Pong, peer_ip: SocketAddr, router: &Router<N>) -> bool {
        // Spawn an asynchronous task for the `Ping` request.
        let router = router.clone();
        let ledger = self.ledger.clone();
        spawn_task!(Self, {
            // Sleep for the preset time before sending a `Ping` request.
            tokio::time::sleep(Duration::from_secs(Router::<N>::PING_SLEEP_IN_SECS)).await;

            // Send a `Ping` request to the peer.
            let message = Message::Ping(Ping {
                version: Message::<N>::VERSION,
                fork_depth: ALEO_MAXIMUM_FORK_DEPTH,
                node_type: Self::NODE_TYPE,
                block_height: Some(ledger.latest_height()),
                status: Self::status().get(),
            });
            if let Err(error) = router.process(RouterRequest::MessageSend(peer_ip, message)).await {
                warn!("[Ping] {error}");
            }
        });
        true
    }

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
    async fn puzzle_response(&self, message: PuzzleResponse<N>, peer_ip: SocketAddr, peer: &Peer<N>) -> bool {
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
                self.latest_epoch_challenge.write().await.replace(epoch_challenge.clone());
                // Save the latest block in the node.
                self.latest_block.write().await.replace(block.clone());
                // Save the latest puzzle response in the node.
                self.latest_puzzle_response
                    .write()
                    .await
                    .replace(PuzzleResponse { epoch_challenge, block: Data::Object(block) });
                // Update the block height of the peer.
                *peer.block_height.write().await = block_height;

                trace!("Received 'PuzzleResponse' from '{peer_ip}' (Epoch {epoch_number}, Block {block_height})");
                true
            }
            Err(error) => {
                error!("Failed to deserialize the puzzle response from '{peer_ip}': {error}");
                false
            }
        }
    }

    /// Attempts to add the unconfirmed block to the ledger. If the block is valid, propagate it to the network.
    async fn unconfirmed_block(
        &self,
        router: &Router<N>,
        peer_ip: SocketAddr,
        message: UnconfirmedBlock<N>,
        block: Block<N>,
    ) -> bool {
        // Ensure the unconfirmed block is at least within 2 blocks of the latest block height,
        // and no more that 2 blocks ahead of the latest block height.
        // If it is stale, skip the routing of this unconfirmed block to the ledger.
        let latest_block_height = self.ledger.latest_height();
        let lower_bound = latest_block_height.saturating_sub(2);
        let upper_bound = latest_block_height.saturating_add(2);
        let is_within_range = block.height() >= lower_bound && block.height() <= upper_bound;

        // Ensure the node is not peering or syncing.
        let is_node_ready = !(Self::status().is_peering() || Self::status().is_syncing());

        if !is_within_range || !is_node_ready {
            trace!("Skipping 'UnconfirmedBlock {}' from {}", block.height(), peer_ip);
            return true;
        } else {
            // Check that the next block is valid.
            if let Err(error) = self.consensus.check_next_block(&block) {
                trace!("[UnconfirmedBlock] {error}");
                return true;
            }

            // Attempt to add the block to the ledger.
            if let Err(error) = self.consensus.advance_to_next_block(&block) {
                trace!("[UnconfirmedBlock] {error}");
                return true;
            }

            info!("Ledger successfully advanced to block {} ({})", block.height(), block.hash());
        }

        // Propagate the `UnconfirmedBlock`.
        let request = RouterRequest::MessagePropagate(Message::UnconfirmedBlock(message), vec![peer_ip]);
        if let Err(error) = router.process(request).await {
            warn!("[UnconfirmedBlock] {error}");
        }
        true
    }

    /// Propagates the unconfirmed solution to all connected beacons.
    async fn unconfirmed_solution(
        &self,
        router: &Router<N>,
        peer_ip: SocketAddr,
        message: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool {
        // Read the latest epoch challenge and latest proof target.
        if let (Some(epoch_challenge), Some(proof_target)) = (
            self.latest_epoch_challenge.read().await.clone(),
            self.latest_block.read().await.as_ref().map(|block| block.proof_target()),
        ) {
            // Ensure that the prover solution is valid for the given epoch.
            match solution.verify(
                self.coinbase_puzzle.coinbase_verifying_key().unwrap(),
                &epoch_challenge,
                proof_target,
            ) {
                Ok(true) => {
                    // Propagate the `UnconfirmedSolution` to connected beacons.
                    let message = Message::UnconfirmedSolution(message);
                    let request = RouterRequest::MessagePropagateBeacon(message, vec![peer_ip]);
                    if let Err(error) = router.process(request).await {
                        warn!("[UnconfirmedSolution] {error}");
                    }
                }
                Ok(false) | Err(_) => {
                    trace!("Invalid prover solution '{}' for the current epoch.", solution.commitment())
                }
            }
        }
        true
    }
}

#[async_trait]
impl<N: Network> Outbound for Validator<N> {}
