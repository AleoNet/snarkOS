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

mod message;
pub use message::*;

use crate::{
    environment::{helpers::NodeType, Environment},
    Ledger,
};

use snarkvm::prelude::*;

use futures::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_util::codec::Framed;

pub type Sender<N> = mpsc::Sender<Message<N>>;
pub type Receiver<N> = mpsc::Receiver<Message<N>>;

pub struct Peer<N: Network> {
    ip: SocketAddr,
    outbound: Framed<TcpStream, MessageCodec<N>>,
    inbound: Receiver<N>,
}

impl<N: Network> Peer<N> {
    async fn new(stream: TcpStream, ledger: Arc<Ledger<N>>) -> io::Result<Self> {
        let outbound = Framed::new(stream, Default::default());
        let addr = outbound.get_ref().peer_addr()?;

        // Create a channel for this peer
        let (outbound_sender, inbound) = mpsc::channel(1024);

        // Send initial ping.
        if let Err(err) = outbound_sender.send(Message::<N>::Ping).await {
            warn!("Failed to send ping {} to {}", err, addr);
        }

        // Store the new peer.
        if ledger.peers().read().contains_key(&addr) {
            return Err(error(format!("Peer {} already exists", addr)));
        } else {
            ledger.peers().write().insert(addr, outbound_sender);
        }

        Ok(Self {
            ip: addr,
            outbound,
            inbound,
        })
    }
}

/// Create a message handler for each peer.
pub(crate) async fn handle_peer<N: Network, E: Environment>(
    stream: TcpStream,
    peer_ip: SocketAddr,
    ledger: Arc<Ledger<N>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut peer = Peer::<N>::new(stream, ledger.clone()).await?;

    info!("Connected to peer: {:?}", peer_ip);

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a peer.
            Some(msg) = peer.inbound.recv() => {
                peer.outbound.send(msg).await?;
            }
            result = peer.outbound.next() => match result {
                // A message was received from the current user, we should
                // broadcast this message to the other users.
                Some(Ok(message)) => {
                    trace!("Received '{}' from {}", message.name(), peer.ip);

                    match message {
                        Message::Ping => {
                            let latest_height = ledger.ledger().read().latest_height();
                            let response = Message::<N>::Pong(latest_height);
                            peer.outbound.send(response).await?;
                        },
                        Message::Pong(height) => {
                            // TODO (raychu86): Handle syncs. Currently just asks for one new block at a time.
                            // If the peer is ahead, ask for next block.
                            let latest_height = ledger.ledger().read().latest_height();
                            if height > latest_height {
                                let request = Message::<N>::BlockRequest(latest_height + 1);
                                peer.outbound.send(request).await?;
                            }
                        },
                        Message::BlockRequest(height) => {
                            let latest_height = ledger.ledger().read().latest_height();
                            if height > latest_height {
                                trace!("Peer requested block {height}, which is greater than the current height {latest_height}");
                            } else {
                                let block = ledger.ledger().read().get_block(height)?;
                                let response = Message::BlockResponse(block);

                                peer.outbound.send(response).await?;
                            }
                        },
                        Message::BlockResponse(block) => {
                            // Check if the block can be added to the ledger.
                            if block.height() == ledger.ledger().read().latest_height() + 1 {
                                // Attempt to add the block to the ledger.
                                match ledger.add_next_block(&block).await {
                                    Ok(_) => info!("Advanced to block {} ({})", block.height(), block.hash()),
                                    Err(err) => warn!("Failed to process block {} (height: {}): {:?}",block.hash(),block.header().height(), err)
                                };

                                // Send a ping.
                                peer.outbound.send(Message::<N>::Ping).await?;
                            } else {
                                trace!("Skipping block {} (height: {})", block.hash(), block.height());
                            }
                        },
                        Message::TransactionBroadcast(transaction) => {
                            let transaction_id = transaction.id();

                            // Check that the transaction doesn't already exist in the ledger or mempool.
                            if let Ok(true) = ledger.ledger().read().contains_transaction_id(&transaction_id) {
                                // Attempt to insert the transaction into the mempool.
                                match ledger.add_to_memory_pool(transaction.clone()) {
                                    Ok(_) => {
                                        // Broadcast transaction to all peers except the sender.
                                        let peers = ledger.peers().read().clone();
                                        tokio::spawn(async move {
                                            for (_, sender) in peers.iter().filter(|(ip, _)| *ip != &peer.ip) {
                                                let _ = sender.send(Message::<N>::TransactionBroadcast(transaction.clone())).await;
                                            }
                                        });

                                    },
                                    Err(err) => {
                                        trace!(
                                            "Failed to add transaction {} to mempool: {:?}",
                                            transaction_id,
                                            err
                                        );
                                    }
                                }
                            }
                        },
                        Message::BlockBroadcast(block) => {
                            // Check if the block can be added to the ledger.
                            if block.height() == ledger.ledger().read().latest_height() + 1 {
                                // Attempt to add the block to the ledger.
                                match ledger.add_next_block(&block).await {
                                    Ok(_) => {
                                        info!("Advanced to block {} ({})", block.height(), block.hash());

                                        // Broadcast block to all peers except the sender.
                                        let peers = ledger.peers().read().clone();
                                        tokio::spawn(async move {
                                            for (_, sender) in peers.iter().filter(|(ip, _)| *ip != &peer.ip) {
                                                let _ = sender.send(Message::<N>::BlockBroadcast(block.clone())).await;
                                            }
                                        });
                                    },
                                     Err(err) => {
                                        trace!(
                                            "Failed to process block {} (height: {}): {:?}",
                                            block.hash(),
                                            block.header().height(),
                                            err
                                        );
                                    }
                                };
                            } else {
                                trace!("Skipping block {} (height: {})", block.hash(), block.height());
                            }
                        },
                        Message::CoinbasePuzzle(_) => {
                            if E::NODE_TYPE == NodeType::Validator || E::NODE_TYPE == NodeType::Beacon {
                                // TODO (raychu86): Verify the coinbase puzzle proof.

                                // TODO (raychu86): Add the coinbase puzzle to a mempool.
                            }
                        }
                    }
                }
                // An error occurred.
                Some(Err(e)) => {
                    warn!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        peer.ip,
                        e
                    );
                }
                // The stream has been exhausted.
                None => {
                    // Remove the peer from the ledger.
                    debug!("Removing connection with peer {}", peer.ip);
                    ledger.peers().write().remove(&peer.ip);
                    return Ok(())},
            },
        }
    }
}

/// Handle connection listener for new peers.
pub async fn handle_listener<N: Network, E: Environment>(
    listener: TcpListener,
    ledger: Arc<Ledger<N>>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("Listening to connections at: {}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        loop {
            let ledger_clone = ledger.clone();

            match listener.accept().await {
                // Process the inbound connection request.
                Ok((stream, peer_ip)) => {
                    tokio::spawn(async move {
                        if let Err(err) = handle_peer::<N, E>(stream, peer_ip, ledger_clone.clone()).await {
                            warn!("Error handling peer {}: {:?}", peer_ip, err);
                        }
                    });
                }
                Err(error) => warn!("Failed to accept a connection: {}", error),
            }
        }
    });

    Ok(())
}

/// Send a ping to all peers every 10 seconds.
pub fn send_pings<N: Network>(ledger: Arc<Ledger<N>>) -> Result<(), Box<dyn std::error::Error>> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;

            let peers = ledger.peers().read().clone();

            for (addr, outbound) in peers.iter() {
                if let Err(err) = outbound.try_send(Message::<N>::Ping) {
                    warn!("Error sending ping {} to {}", err, addr);
                }
            }
        }
    });

    Ok(())
}

/// Handle connection with the leader.
pub async fn connect_to_leader<N: Network, E: Environment>(
    leader_addr: SocketAddr,
    ledger: Arc<Ledger<N>>,
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(10));
        loop {
            let ledger_clone = ledger.clone();

            if !ledger_clone.peers().read().contains_key(&leader_addr) {
                trace!("Attempting to connect to peer {}", leader_addr);
                match TcpStream::connect(leader_addr).await {
                    Ok(stream) => {
                        tokio::spawn(async move {
                            if let Err(err) = handle_peer::<N, E>(stream, leader_addr, ledger_clone.clone()).await {
                                warn!("Error handling peer {}: {:?}", leader_addr, err);
                            } else {
                                // TODO (raychu86): Dynamically update validators. Currently only the beacon acts as a validator.
                                ledger_clone.validators().write().insert(leader_addr);
                            }
                        });
                    }
                    Err(error) => warn!("Failed to connect to peer {}: {}", leader_addr, error),
                }
            }
            interval.tick().await;
        }
    });

    Ok(())
}
