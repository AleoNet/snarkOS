// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;

use snarkos_node_bft_consensus::{batched_transactions, sort_transactions};
use snarkos_node_messages::{
    BlockRequest,
    BlockResponse,
    ConsensusId,
    Data,
    DataBlocks,
    DisconnectReason,
    Message,
    MessageCodec,
    NewBlock,
    Pong,
    UnconfirmedTransaction,
};
use snarkos_node_tcp::{Connection, ConnectionSide, Tcp};
use snarkvm::prelude::{error, EpochChallenge, Network, Transaction};

use bytes::BytesMut;
use futures_util::sink::SinkExt;
use narwhal_executor::ExecutionState;
use std::{collections::HashSet, io, net::SocketAddr, time::Duration};
use tokio::{net::TcpStream, task::spawn_blocking};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

impl<N: Network, C: ConsensusStorage<N>> P2P for Validator<N, C> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        self.router.tcp()
    }
}

impl<N: Network, C: ConsensusStorage<N>> Validator<N, C> {
    /// An extended handshake dedicated to consensus committee members.
    async fn committee_handshake(
        &self,
        peer_ip: SocketAddr,
        mut framed: Framed<&mut TcpStream, MessageCodec<N>>,
    ) -> io::Result<()> {
        // Establish quorum with other validators:
        //
        // 1. Sign and send the node's pub key.
        // 2. Receive and verify peer's signed pub key.
        // 3. Insert into connected_committee_members.
        // 4. If quorum threshold is reached, start the bft.

        // 1.
        // BFT must be set here.
        // TODO: we should probably use something else than the public key, potentially interactive, since this could
        // be copied and reused by a malicious validator.
        let public_key = self.primary_keypair.public();
        let signature = self
            .primary_keypair
            .private()
            .sign_bytes(public_key.to_bytes().as_slice(), &mut rand::thread_rng())
            .unwrap();

        let last_executed_sub_dag_index =
            if let Some(bft) = self.bft.get() { bft.state.last_executed_sub_dag_index().await } else { 0 };
        let message = Message::ConsensusId(Box::new(ConsensusId {
            public_key: public_key.clone(),
            signature,
            last_executed_sub_dag_index,
            aleo_address: self.address(),
        }));
        framed.send(message).await?;

        // 2.
        let consensus_id = match framed.try_next().await? {
            Some(Message::ConsensusId(data)) => data,
            _ => return Err(error(format!("'{peer_ip}' did not send a 'ConsensusId' message"))),
        };

        // Check the advertised public key exists in the committee.
        if !self.committee.keys().contains(&&consensus_id.public_key) {
            return Err(error(format!("'{peer_ip}' is not part of the committee")));
        }

        // Check the signature.
        // TODO: again, the signed message should probably be something we send to the peer, not
        // their public key.
        if !consensus_id.signature.verify_bytes(&consensus_id.public_key, &consensus_id.public_key.to_bytes()) {
            return Err(error(format!("'{peer_ip}' couldn't verify their identity")));
        }

        // 3.
        // Track the committee member.
        // TODO: in future we could error here if it already exists in the collection but that
        // logic is probably best implemented when dynamic committees are being considered.
        self.connected_committee_members.write().insert(peer_ip, consensus_id.public_key);

        // 3.5
        // add the peer to the ledger's `current_committee`
        self.ledger.insert_committee_member(consensus_id.aleo_address);

        // 4.
        // If quorum is reached, start the consensus but only if it hasn't already been started.
        let own_stake = self.committee.stake(public_key);
        let connected_stake =
            self.connected_committee_members.read().values().map(|pk| self.committee.stake(pk)).sum::<u64>();
        if own_stake + connected_stake >= self.committee.quorum_threshold() && self.bft.get().is_none() {
            self.start_bft(consensus_id.last_executed_sub_dag_index).await.unwrap()
        }

        Ok(())
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Handshake for Validator<N, C> {
    /// Performs the handshake protocol.
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Perform the handshake.
        let peer_addr = connection.addr();
        let conn_side = connection.side();
        let stream = self.borrow_stream(&mut connection);
        let genesis_header = self.ledger.get_header(0).map_err(|e| error(format!("{e}")))?;
        let (peer, framed) = self.router.handshake(peer_addr, stream, conn_side, genesis_header).await?;

        // If both nodes are validators, perform the extended committee handshake.
        if peer.node_type() == NodeType::Validator {
            let handshake_result = self.committee_handshake(peer.ip(), framed).await;
            // Adjust the list of connecting peers regardless of the result.
            self.router.connecting_peers.lock().remove(&peer.ip());
            handshake_result?;
        }

        // In case of success, announce it and insert the peer.
        info!("Connected to '{}'", peer.ip());
        self.router.insert_connected_peer(peer, peer_addr);

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> OnConnect for Validator<N, C>
where
    Self: Outbound<N>,
{
    async fn on_connect(&self, peer_addr: SocketAddr) {
        let peer_ip = if let Some(ip) = self.router.resolve_to_listener(&peer_addr) {
            ip
        } else {
            return;
        };

        // Retrieve the block locators.
        let block_locators = match crate::helpers::get_block_locators(&self.ledger) {
            Ok(block_locators) => Some(block_locators),
            Err(e) => {
                error!("Failed to get block locators: {e}");
                return;
            }
        };

        // Send the first `Ping` message to the peer.
        self.send_ping(peer_ip, block_locators);
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Disconnect for Validator<N, C> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        if let Some(peer_ip) = self.router.resolve_to_listener(&peer_addr) {
            self.router.remove_connected_peer(peer_ip);
            self.connected_committee_members.write().remove(&peer_ip);
        }
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Writing for Validator<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Reading for Validator<N, C> {
    type Codec = MessageCodec<N>;
    type Message = Message<N>;

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    /// Processes a message received from the network.
    async fn process_message(&self, peer_addr: SocketAddr, message: Self::Message) -> io::Result<()> {
        // Process the message. Disconnect if the peer violated the protocol.
        if let Err(error) = self.inbound(peer_addr, message).await {
            if let Some(peer_ip) = self.router().resolve_to_listener(&peer_addr) {
                warn!("Disconnecting from '{peer_ip}' - {error}");
                self.send(peer_ip, Message::Disconnect(DisconnectReason::ProtocolViolation.into()));
                // Disconnect from this peer.
                self.router().disconnect(peer_ip);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Routing<N> for Validator<N, C> {}

impl<N: Network, C: ConsensusStorage<N>> Heartbeat<N> for Validator<N, C> {
    /// The maximum number of peers permitted to maintain connections with.
    const MAXIMUM_NUMBER_OF_PEERS: usize = 1_000;
}

impl<N: Network, C: ConsensusStorage<N>> Outbound<N> for Validator<N, C> {
    /// Returns a reference to the router.
    fn router(&self) -> &Router<N> {
        &self.router
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> Inbound<N> for Validator<N, C> {
    /// Retrieves the blocks within the block request range, and returns the block response to the peer.
    fn block_request(&self, peer_ip: SocketAddr, message: BlockRequest) -> bool {
        let BlockRequest { start_height, end_height } = &message;

        // Retrieve the blocks within the requested range.
        let blocks = match self.ledger.get_blocks(*start_height..*end_height) {
            Ok(blocks) => Data::Object(DataBlocks(blocks)),
            Err(error) => {
                error!("Failed to retrieve blocks {start_height} to {end_height} from the ledger - {error}");
                return false;
            }
        };
        // Send the `BlockResponse` message to the peer.
        self.send(peer_ip, Message::BlockResponse(BlockResponse { request: message, blocks }));
        true
    }

    /// Handles a `BlockResponse` message.
    fn block_response(&self, peer_ip: SocketAddr, blocks: Vec<Block<N>>) -> bool {
        // Insert the candidate blocks into the sync pool.
        for block in blocks {
            if let Err(error) = self.router().sync().insert_block_response(peer_ip, block) {
                warn!("{error}");
                return false;
            }
        }

        // Tries to advance with blocks from the sync pool.
        self.advance_with_sync_blocks();
        true
    }

    /// Handles a `NewBlock` message.
    fn new_block(&self, peer_ip: SocketAddr, block: Block<N>, serialized: NewBlock<N>) -> bool {
        // If the BFT isn't ready, we can't process the block yet.
        if self.bft.get().is_none() {
            return true;
        }

        if self.processed_block.read().as_ref().map(|b| b.height()) == Some(block.height()) {
            return true;
        } else {
            *self.processed_block.write() = Some(block.clone());
        }

        // A failed check doesn't necessarily mean the block is malformed, so return true here.
        if self.consensus.check_next_block(&block).is_err() {
            *self.processed_block.write() = None;
            return true;
        }

        // If the previous consensus output is available, check the order of transactions.
        if let Some(last_consensus_output) = self.bft().state.last_output.lock().clone() {
            let mut expected_txs = batched_transactions(&last_consensus_output)
                .map(|bytes| {
                    // Safe; it's our own consensus output, so we already processed this tx with the TransactionValidator.
                    // Also, it's fast to deserialize, because we only process the ID and keep the actual tx as a blob.
                    // This, of course, assumes that only the ID is used for sorting.
                    let message = Message::<N>::deserialize(BytesMut::from(&bytes[..])).unwrap();

                    let unconfirmed_tx = if let Message::UnconfirmedTransaction(tx) = message {
                        tx
                    } else {
                        // TransactionValidator ensures that the Message is an UnconfirmedTransaction.
                        unreachable!();
                    };

                    unconfirmed_tx.transaction_id
                })
                .collect::<HashSet<_>>();

            // Remove the ids that are not present in the block (presumably dropped due to ledger rejection).
            let block_txs = block.transaction_ids().copied().collect::<HashSet<_>>();
            for id in &expected_txs.clone() {
                if !block_txs.contains(id) {
                    expected_txs.remove(id);
                }
            }

            // Sort the txs according to shared logic.
            let mut expected_txs = expected_txs.into_iter().collect::<Vec<_>>();
            sort_transactions::<N>(&mut expected_txs);

            if block.transaction_ids().zip(&expected_txs).any(|(id1, id2)| id1 != id2) {
                error!("[NewBlock] Invalid order of transactions");
                *self.processed_block.write() = None;
                return false;
            }
        }

        // Attempt to add the block to the ledger.
        if let Err(err) = self.consensus.advance_to_next_block(&block) {
            error!("[NewBlock] {err}");
            *self.processed_block.write() = None;
            return false;
        }

        *self.processed_block.write() = None;

        // TODO: perform more elaborate propagation
        self.propagate(Message::NewBlock(serialized), &[peer_ip]);

        true
    }

    /// Sleeps for a period and then sends a `Ping` message to the peer.
    fn pong(&self, peer_ip: SocketAddr, _message: Pong) -> bool {
        // Spawn an asynchronous task for the `Ping` request.
        let self_clone = self.clone();
        tokio::spawn(async move {
            // Sleep for the preset time before sending a `Ping` request.
            tokio::time::sleep(Duration::from_secs(Self::PING_SLEEP_IN_SECS)).await;
            // Check that the peer is still connected.
            if self_clone.router().is_connected(&peer_ip) {
                // Retrieve the block locators.
                match crate::helpers::get_block_locators(&self_clone.ledger) {
                    // Send a `Ping` message to the peer.
                    Ok(block_locators) => self_clone.send_ping(peer_ip, Some(block_locators)),
                    Err(e) => error!("Failed to get block locators: {e}"),
                }
            }
        });
        true
    }

    /// Retrieves the latest epoch challenge and latest block header, and returns the puzzle response to the peer.
    fn puzzle_request(&self, peer_ip: SocketAddr) -> bool {
        // Retrieve the latest epoch challenge.
        let epoch_challenge = match self.ledger.latest_epoch_challenge() {
            Ok(epoch_challenge) => epoch_challenge,
            Err(error) => {
                error!("Failed to prepare a puzzle request for '{peer_ip}': {error}");
                return false;
            }
        };
        // Retrieve the latest block header.
        let block_header = Data::Object(self.ledger.latest_header());
        // Send the `PuzzleResponse` message to the peer.
        self.send(peer_ip, Message::PuzzleResponse(PuzzleResponse { epoch_challenge, block_header }));
        true
    }

    /// Disconnects on receipt of a `PuzzleResponse` message.
    fn puzzle_response(&self, peer_ip: SocketAddr, _epoch_challenge: EpochChallenge<N>, _header: Header<N>) -> bool {
        debug!("Disconnecting '{peer_ip}' for the following reason - {:?}", DisconnectReason::ProtocolViolation);
        false
    }

    /// Propagates the unconfirmed solution to all connected beacons and validators.
    async fn unconfirmed_solution(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedSolution<N>,
        solution: ProverSolution<N>,
    ) -> bool {
        // Add the unconfirmed solution to the memory pool.
        let node = self.clone();
        match spawn_blocking(move || node.consensus.add_unconfirmed_solution(&solution)).await {
            Ok(Err(error)) => {
                trace!("[UnconfirmedSolution] {error}");
                return true; // Maintain the connection.
            }
            Err(error) => {
                trace!("[UnconfirmedSolution] {error}");
                return true; // Maintain the connection.
            }
            _ => {}
        }
        let message = Message::UnconfirmedSolution(serialized);
        // Propagate the "UnconfirmedSolution" to the connected beacons.
        self.propagate_to_beacons(message.clone(), &[peer_ip]);
        // Propagate the "UnconfirmedSolution" to the connected validators.
        self.propagate_to_validators(message, &[peer_ip]);
        true
    }

    /// Handles an `UnconfirmedTransaction` message.
    fn unconfirmed_transaction(
        &self,
        peer_ip: SocketAddr,
        serialized: UnconfirmedTransaction<N>,
        _transaction: Transaction<N>,
    ) -> bool {
        let message = Message::UnconfirmedTransaction(serialized);
        // Propagate the "UnconfirmedTransaction" to the connected beacons.
        self.propagate_to_beacons(message.clone(), &[peer_ip]);
        // Propagate the "UnconfirmedTransaction" to the connected validators.
        self.propagate_to_validators(message, &[peer_ip]);
        true
    }
}
