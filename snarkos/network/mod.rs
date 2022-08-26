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

use crate::Ledger;

use snarkvm::prelude::*;

use futures::{SinkExt, StreamExt};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task,
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
pub(crate) async fn handle_peer<N: Network>(
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
                                let response = Message::BlockResponse(Data::Object(block));

                                peer.outbound.send(response).await?;
                            }
                        },
                        Message::BlockResponse(block_bytes) => {
                            // Perform deferred deserialization.
                            let block = block_bytes.deserialize().await?;

                            let block_height = block.height();
                            let block_hash = block.hash();

                            // Check if the block can be added to the ledger.
                            if block_height == ledger.ledger().read().latest_height() + 1 {
                                // Attempt to add the block to the ledger.
                                match ledger.add_next_block(block).await {
                                    Ok(_) => info!("Advanced to block {} ({})", block_height, block_hash),
                                    Err(err) => warn!("Failed to process block {} (height: {}): {:?}", block_hash, block_height, err)
                                };

                                // Send a ping.
                                peer.outbound.send(Message::<N>::Ping).await?;
                            } else {
                                trace!("Skipping block {} (height: {})", block_hash, block_height);
                            }
                        },
                        Message::TransactionBroadcast(transaction_bytes) => {
                            // Perform deferred deserialization.
                            let transaction = transaction_bytes.clone().deserialize().await?;

                            let transaction_id = transaction.id();

                            // Check that the transaction doesn't already exist in the ledger or mempool.
                            if let Ok(true) = ledger.ledger().read().contains_transaction_id(&transaction_id) {
                                // Attempt to insert the transaction into the mempool.
                                match ledger.add_to_memory_pool(transaction) {
                                    Ok(_) => {
                                        // Broadcast transaction to all peers except the sender.
                                        let peers = ledger.peers().read().clone();
                                        tokio::spawn(async move {
                                            for (_, sender) in peers.iter().filter(|(ip, _)| *ip != &peer.ip) {
                                                let _ = sender.send(Message::<N>::TransactionBroadcast(transaction_bytes.clone())).await;
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
                        Message::BlockBroadcast(block_bytes) => {
                            // Perform deferred deserialization.
                            let block = block_bytes.clone().deserialize().await?;

                            let block_height = block.height();
                            let block_hash = block.hash();

                            // Check if the block can be added to the ledger.
                            if block_height == ledger.ledger().read().latest_height() + 1 {
                                // Attempt to add the block to the ledger.
                                match ledger.add_next_block(block).await {
                                    Ok(_) => {
                                        info!("Advanced to block {} ({})", block_height, block_hash);

                                        // Broadcast block to all peers except the sender.
                                        let peers = ledger.peers().read().clone();
                                        tokio::spawn(async move {
                                            for (_, sender) in peers.iter().filter(|(ip, _)| *ip != &peer.ip) {
                                                let _ = sender.send(Message::<N>::BlockBroadcast(block_bytes.clone())).await;
                                            }
                                        });
                                    },
                                     Err(err) => {
                                        trace!(
                                            "Failed to process block {} (height: {}): {:?}",
                                            block_hash,
                                            block_height,
                                            err
                                        );
                                    }
                                };
                            } else {
                                trace!("Skipping block {} (height: {})", block_hash, block_height);
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
pub fn handle_listener<N: Network>(listener: TcpListener, ledger: Arc<Ledger<N>>) -> task::JoinHandle<()> {
    info!("Listening to connections at: {}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        loop {
            let ledger_clone = ledger.clone();

            match listener.accept().await {
                // Process the inbound connection request.
                Ok((stream, peer_ip)) => {
                    tokio::spawn(async move {
                        if let Err(err) = handle_peer::<N>(stream, peer_ip, ledger_clone.clone()).await {
                            warn!("Error handling peer {}: {:?}", peer_ip, err);
                        }
                    });
                }
                Err(error) => warn!("Failed to accept a connection: {}", error),
            }
        }
    })
}

// TODO (raychu86): Handle this request via `Message::BlockRequest`. This is currently not done,
//  because the node has not established the leader as a peer.
/// Request the genesis block from the leader.
pub(super) async fn request_genesis_block<N: Network>(leader_ip: IpAddr) -> Result<Block<N>> {
    info!("Requesting genesis block from {}", leader_ip);
    let block_string = reqwest::get(format!("http://{leader_ip}/testnet3/block/0")).await?.text().await?;

    Block::from_str(&block_string)
}

/// Send a ping to all peers every 10 seconds.
pub fn send_pings<N: Network>(ledger: Arc<Ledger<N>>) -> task::JoinHandle<()> {
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
    })
}

/// Handle connection with the leader.
pub fn connect_to_leader<N: Network>(initial_peer: SocketAddr, ledger: Arc<Ledger<N>>) -> task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(10));
        loop {
            if !ledger.peers().read().contains_key(&initial_peer) {
                trace!("Attempting to connect to peer {}", initial_peer);
                match TcpStream::connect(initial_peer).await {
                    Ok(stream) => {
                        let ledger_clone = ledger.clone();
                        tokio::spawn(async move {
                            if let Err(err) = handle_peer::<N>(stream, initial_peer, ledger_clone).await {
                                warn!("Error handling peer {}: {:?}", initial_peer, err);
                            }
                        });
                    }
                    Err(error) => warn!("Failed to connect to peer {}: {}", initial_peer, error),
                }
            }
            interval.tick().await;
        }
    })
}
