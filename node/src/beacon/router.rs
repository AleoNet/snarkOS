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
impl<N: Network> Handshake for Beacon<N> {
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 10;
}

#[async_trait]
impl<N: Network> Inbound<N> for Beacon<N> {
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
        // Retrieve the latest epoch challenge and latest block.
        let (epoch_challenge, block) = {
            // Retrieve the latest epoch challenge.
            let epoch_challenge = match self.ledger.latest_epoch_challenge() {
                Ok(block) => block,
                Err(error) => {
                    error!("Failed to retrieve latest epoch challenge for a puzzle request: {error}");
                    return false;
                }
            };
            // Retrieve the latest block.
            let block = self.ledger.latest_block();

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
        _router: &Router<N>,
        _peer_ip: SocketAddr,
        _message: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool {
        // Add the unconfirmed solution to the memory pool.
        if let Err(error) = self.consensus.add_unconfirmed_solution(&solution) {
            trace!("[UnconfirmedSolution] {error}");
            return true; // Maintain the connection.
        }
        // // Propagate the `UnconfirmedSolution` to connected beacons.
        // let request = RouterRequest::MessagePropagateBeacon(Message::UnconfirmedSolution(message), vec![peer_ip]);
        // if let Err(error) = router.process(request).await {
        //     warn!("[UnconfirmedSolution] {error}");
        // }
        true
    }

    /// Adds the unconfirmed transaction to the memory pool, and propagates the transaction to all peers.
    async fn unconfirmed_transaction(
        &self,
        _router: &Router<N>,
        _peer_ip: SocketAddr,
        _message: UnconfirmedTransaction<N>,
        transaction: Transaction<N>,
    ) -> bool {
        // Add the unconfirmed transaction to the memory pool.
        if let Err(error) = self.consensus.add_unconfirmed_transaction(transaction) {
            trace!("[UnconfirmedTransaction] {error}");
            return true; // Maintain the connection.
        }
        // // Propagate the `UnconfirmedTransaction`.
        // let request = RouterRequest::MessagePropagate(Message::UnconfirmedTransaction(message), vec![peer_ip]);
        // if let Err(error) = router.process(request).await {
        //     warn!("[UnconfirmedTransaction] {error}");
        // }
        true
    }
}

#[async_trait]
impl<N: Network> Outbound for Beacon<N> {}
