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
use crate::{
    environment::{helpers::NodeType, Environment},
    ledger::LedgerRequest,
    Data,
    DisconnectReason,
    LedgerRouter,
    Message,
    MessageCodec,
    PeersRequest,
    PeersRouter,
};

use snarkvm::prelude::*;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use indexmap::IndexMap;
use std::{
    marker::PhantomData,
    net::SocketAddr,
    time::{Duration, Instant, SystemTime},
};
use tokio::{net::TcpStream, sync::mpsc};
use tokio_util::codec::Framed;

/// Shorthand for the parent half of the `Peer` outbound message channel.
pub type OutboundRouter<N> = mpsc::Sender<Message<N>>;
/// Shorthand for the child half of the `Peer` outbound message channel.
pub type OutboundHandler<N> = mpsc::Receiver<Message<N>>;

///
/// The state for each connected client.
///
pub(crate) struct Peer<N: Network, E: Environment> {
    /// The IP address of the peer, with the port set to the listener port.
    listener_ip: SocketAddr,
    // TODO (raychu86): Introduce message version.
    // /// The message version of the peer.
    // version: u32,
    /// The timestamp of the last message received from this peer.
    last_seen: Instant,
    /// The TCP socket that handles sending and receiving data with this peer.
    outbound_socket: Framed<TcpStream, MessageCodec<N>>,
    /// The `outbound_handler` half of the MPSC message channel, used to receive messages from peers.
    /// When a message is received on this `OutboundHandler`, it will be written to the socket.
    outbound_handler: OutboundHandler<N>,
    /// The map of transaction IDs to their last seen timestamp.
    seen_inbound_transactions: IndexMap<N::TransactionID, SystemTime>,
    /// The map of peers to a map of transaction IDs to their last seen timestamp.
    seen_outbound_transactions: IndexMap<N::TransactionID, SystemTime>,
    _phantom: PhantomData<E>,
}

impl<N: Network, E: Environment> Peer<N, E> {
    /// Create a new instance of `Peer`.
    async fn new(stream: TcpStream, peers_router: &PeersRouter<N>) -> Result<Self> {
        // Construct the socket.
        let outbound_socket = Framed::new(stream, Default::default());
        let peer_ip = outbound_socket.get_ref().peer_addr()?;

        // Create a channel for this peer.
        let (outbound_router, outbound_handler) = mpsc::channel(1024);

        // Send the first `Ping` message to the peer.
        if let Err(err) = outbound_router.send(Message::<N>::Ping).await {
            warn!("Failed to send ping {} to {}", err, peer_ip);
        }

        // Add an entry for this `Peer` in the connected peers.
        peers_router.send(PeersRequest::PeerConnected(peer_ip, outbound_router)).await?;

        Ok(Self {
            listener_ip: peer_ip,
            last_seen: Instant::now(),
            outbound_socket,
            outbound_handler,
            seen_inbound_transactions: Default::default(),
            seen_outbound_transactions: Default::default(),
            _phantom: PhantomData,
        })
    }

    /// Returns the IP address of the peer, with the port set to the listener port.
    fn peer_ip(&self) -> SocketAddr {
        self.listener_ip
    }

    /// Sends the given message to this peer.
    async fn send(&mut self, message: Message<N>) -> Result<()> {
        trace!("Sending '{}' to {}", message.name(), self.peer_ip());
        self.outbound_socket.send(message).await?;
        Ok(())
    }

    /// A handler to process an individual peer.
    pub async fn handler(stream: TcpStream, peer_ip: SocketAddr, peers_router: PeersRouter<N>, ledger_router: LedgerRouter<N>) {
        E::resources().register_task(
            None,
            tokio::spawn(async move {
                // Register our peer with state which internally sets up some channels.
                let mut peer = match Peer::<N, E>::new(stream, &peers_router).await {
                    Ok(peer) => peer,
                    Err(err) => {
                        warn!("Failed to register peer {}: {}", peer_ip, err);
                        return;
                    }
                };

                // Retrieve the peer IP.
                info!("Connected to {}", peer_ip);

                // Process incoming messages until this stream is disconnected.
                loop {
                    tokio::select! {
                // Message channel is routing a message outbound to the peer.
                Some(mut message) = peer.outbound_handler.recv() => {
                    // Disconnect if the peer has not communicated back within the predefined time.
                    if peer.last_seen.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                        warn!("Peer {} has not communicated in {} seconds", peer_ip, peer.last_seen.elapsed().as_secs());
                        break;
                    } else {
                        // Ensure sufficient time has passed before needing to send the message.
                        let is_ready_to_send = match message {
                            Message::Ping => {
                                true
                            }
                            Message::TransactionBroadcast(ref mut data) => {
                                let transaction = if let Data::Object(transaction) = data {
                                    transaction
                                } else {
                                    panic!("Logic error: the transaction shouldn't have been serialized yet.");
                                };

                                // Retrieve the last seen timestamp of this transaction for this peer.
                                let last_seen = peer
                                    .seen_outbound_transactions
                                    .entry(transaction.id())
                                    .or_insert(SystemTime::UNIX_EPOCH);
                                let is_ready_to_send = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                // Update the timestamp for the peer and sent transaction.
                                peer.seen_outbound_transactions.insert(transaction.id(), SystemTime::now());
                                // Report the unconfirmed block height.
                                if is_ready_to_send {
                                    trace!(
                                        "Preparing to send 'TransactionBroadcast {}' to {}",
                                        transaction.id(),
                                        peer_ip
                                    );
                                }

                                // Perform non-blocking serialization of the transaction.
                                let serialized_transaction = Data::serialize(data.clone()).await.expect("Transaction serialization is bugged");
                                let _ = std::mem::replace(data, Data::Buffer(serialized_transaction));

                                is_ready_to_send
                            }
                            _ => true,
                        };
                        // Send the message if it is ready.
                        if is_ready_to_send {
                            // Route a message to the peer.
                            if let Err(error) = peer.send(message).await {
                                warn!("[OutboundRouter] {}", error);
                            }
                        }
                    }
                }
                result = peer.outbound_socket.next() => match result {
                    // Received a message from the peer.
                    Some(Ok(message)) => {
                        // Disconnect if the peer has not communicated back within the predefined time.
                        match peer.last_seen.elapsed() > Duration::from_secs(E::RADIO_SILENCE_IN_SECS) {
                            true => {
                                let last_seen = peer.last_seen.elapsed().as_secs();
                                warn!("Failed to receive a message from {} in {} seconds", peer_ip, last_seen);
                                break;
                            },
                            false => {
                                // Update the last seen timestamp.
                                peer.last_seen = Instant::now();
                            }
                        }
                        // Process the message.
                        trace!("Received '{}' from {}", message.name(), peer_ip);
                        match message {
                            Message::Ping => {
                                // Route the `Ping` to the ledger.
                                if let Err(error) = ledger_router.send(LedgerRequest::Ping(peer_ip)).await {
                                    warn!("[Ping] {}", error);
                                }
                            }
                            Message::Pong(height) => {
                                // Route the request to the ledger.
                                if let Err(error) = ledger_router.send(LedgerRequest::Pong(peer_ip, height, None)).await {
                                    warn!("[Pong] {}", error);
                                }
                            },
                            Message::Disconnect(reason) => {
                                debug!("Peer {} disconnected for the following reason: {:?}", peer_ip, reason);
                                break;
                            },
                            Message::BlockRequest(height) => {
                                // Route the request to the ledger.
                                if let Err(error) = ledger_router.send(LedgerRequest::BlockRequest(peer_ip, height)).await {
                                    warn!("[BlockRequest] {}", error);
                                }
                            },
                            Message::BlockResponse(block_bytes) => {
                                // Perform the deferred non-blocking deserialization of the block.
                                match block_bytes.deserialize().await {
                                    Ok(block) => {
                                        // Route the `BlockResponse` to the ledger.
                                        if let Err(error) = ledger_router.send(LedgerRequest::BlockResponse(peer_ip, block)).await {
                                            warn!("[BlockResponse] {}", error);
                                        }
                                    },
                                    // Route the `Failure` to the ledger.
                                    Err(error) => if let Err(error) = ledger_router.send(LedgerRequest::Failure(peer_ip, format!("{}", error))).await {
                                        warn!("[Failure] {}", error);
                                    }
                                }
                            }

                            Message::TransactionBroadcast(transaction_bytes) => {
                                // Drop the peer, if they have sent more than 500 unconfirmed transactions in the last 5 seconds.
                                let frequency = peer.seen_inbound_transactions.values().filter(|t| t.elapsed().unwrap().as_secs() <= 5).count();
                                if frequency >= 500 {
                                    warn!("Dropping {} for spamming unconfirmed transactions (frequency = {})", peer_ip, frequency);
                                    // Send a `PeerRestricted` message.
                                    if let Err(error) = peers_router.send(PeersRequest::PeerRestricted(peer_ip)).await {
                                        warn!("[PeerRestricted] {}", error);
                                    }
                                    break;
                                }

                                // Perform the deferred non-blocking deserialization of the
                                // transaction.
                                match transaction_bytes.clone().deserialize().await {
                                    Ok(transaction) => {
                                        // Retrieve the last seen timestamp of the received transaction.
                                        let last_seen = peer.seen_inbound_transactions.entry(transaction.id()).or_insert(SystemTime::UNIX_EPOCH);
                                        let is_router_ready = last_seen.elapsed().unwrap().as_secs() > E::RADIO_SILENCE_IN_SECS;

                                        // Update the timestamp for the received transaction.
                                        peer.seen_inbound_transactions.insert(transaction.id(), SystemTime::now());

                                        let transaction_id = transaction.id();
                                        if E::NODE_TYPE == NodeType::Beacon || !is_router_ready {
                                            trace!("Skipping 'TransactionBroadcast {}' from {}", transaction_id, peer_ip);
                                        } else {
                                             // Route the `TransactionBroadcast` to the ledger.
                                            if let Err(error) = ledger_router.send(LedgerRequest::UnconfirmedTransaction(peer_ip, transaction)).await {
                                                warn!("[TransactionBroadcast] {}", error);

                                            }
                                        }

                                    }
                                    Err(error) => warn!("[TransactionBroadcast] {}", error)
                                }
                            }
                            Message::BlockBroadcast(block_bytes) => {
                                // Perform the deferred non-blocking deserialization of the block.
                                let request = match block_bytes.deserialize().await {
                                        // Route the `UnconfirmedBlock` to the ledger.
                                    Ok(block) => LedgerRequest::UnconfirmedBlock(peer_ip, block),
                                    // Route the `Failure` to the ledger.
                                    Err(error) => LedgerRequest::Failure(peer_ip, format!("{}", error)),
                                };

                                // Route the request to the ledger.
                                if let Err(error) = ledger_router.send(request).await {
                                    warn!("[BlockBroadcast] {}", error);
                                }
                            }
                        }
                    }
                    // An error occurred.
                    Some(Err(error)) => error!("Failed to read message from {}: {}", peer_ip, error),
                    // The stream has been disconnected.
                    None => break,
                },
            }
                }

                // When this is reached, it means the peer has disconnected.
                // Route a `Disconnect` to the ledger.
                if let Err(error) = ledger_router
                    .send(LedgerRequest::Disconnect(peer_ip, DisconnectReason::PeerHasDisconnected))
                    .await
                {
                    warn!("[Peer::Disconnect] {}", error);
                }
            }),
        );
    }
}
