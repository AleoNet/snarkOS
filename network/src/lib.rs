// Copyright (C) 2019-2021 Aleo Systems Inc.
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

// Compilation
#![allow(clippy::module_inception)]
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
// Documentation
#![cfg_attr(nightly, feature(doc_cfg, external_doc))]
#![cfg_attr(nightly, doc(include = "../documentation/concepts/network_server.md"))]

#[macro_use]
extern crate thiserror;

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate snarkos_metrics;

pub mod external;

pub mod blocks;
pub use blocks::*;

pub mod environment;
pub use environment::*;

pub mod errors;
pub use errors::*;

pub mod inbound;
pub use inbound::*;

pub mod outbound;
pub use outbound::*;

pub mod peers;
pub use peers::*;

pub mod transactions;
pub use transactions::*;

use crate::{external::message::*, peers::peers::Peers, ConnWriter};

use parking_lot::RwLock;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{task, time::sleep};

pub const HANDSHAKE_PATTERN: &str = "Noise_XXpsk3_25519_ChaChaPoly_SHA256";
pub const HANDSHAKE_PSK: &[u8] = b"b765e427e836e0029a1e2a22ba60c52a"; // the PSK must be 32B
pub const MAX_MESSAGE_SIZE: usize = 8 * 1024 * 1024; // 8MiB
pub const NOISE_BUF_LEN: usize = 65535;
pub const NOISE_TAG_LEN: usize = 16;

pub(crate) type Sender = tokio::sync::mpsc::Sender<Message>;

pub(crate) type Receiver = tokio::sync::mpsc::Receiver<Message>;

/// A core data structure for operating the networking stack of this node.
#[derive(Clone)]
pub struct Server {
    /// The parameters and settings of this node server.
    pub environment: Environment,
    /// The inbound handler of this node server.
    inbound: Arc<Inbound>,
    /// The outbound handler of this node server.
    outbound: Arc<Outbound>,

    pub peers: Peers,
    pub blocks: Blocks,
    pub transactions: Transactions,
}

impl Server {
    /// Creates a new instance of `Server`.
    pub async fn new(environment: Environment) -> Result<Self, NetworkError> {
        let channels: Arc<RwLock<HashMap<SocketAddr, Arc<ConnWriter>>>> = Default::default();
        // Create the inbound and outbound handlers.
        let inbound = Arc::new(Inbound::new(channels.clone()));
        let outbound = Arc::new(Outbound::new(channels));

        // Initialize the peer and block services.
        let peers = Peers::new(environment.clone(), inbound.clone(), outbound.clone())?;
        let blocks = Blocks::new(environment.clone(), outbound.clone());
        let transactions = Transactions::new(environment.clone(), outbound.clone());

        Ok(Self {
            environment,
            inbound,
            outbound,
            peers,
            blocks,
            transactions,
        })
    }

    pub async fn establish_address(&mut self) -> Result<(), NetworkError> {
        self.inbound.listen(&mut self.environment).await?;
        let address = self.environment.local_address().unwrap();

        // update the local address for Blocks and Peers
        self.peers.environment.set_local_address(address);
        self.blocks.environment.set_local_address(address);

        Ok(())
    }

    pub async fn start_services(&self) {
        let peer_sync_interval = self.environment.peer_sync_interval();
        let peers = self.peers.clone();
        task::spawn(async move {
            loop {
                sleep(peer_sync_interval).await;
                info!("Updating peers");

                if let Err(e) = peers.update().await {
                    error!("Peer update error: {}", e);
                }
            }
        });

        if self.environment.has_consensus() && !self.environment.is_bootnode() {
            let peers = self.peers.clone();
            let blocks = self.blocks.clone();
            let block_sync_interval = self.environment.block_sync_interval();
            task::spawn(async move {
                loop {
                    sleep(block_sync_interval).await;
                    info!("Updating blocks");

                    // select last seen node as block sync node
                    let sync_node = peers.last_seen();
                    blocks.update(sync_node).await;
                }
            });

            let peers = self.peers.clone();
            let transactions = self.transactions.clone();
            let transaction_sync_interval = self.environment.transaction_sync_interval();
            task::spawn(async move {
                loop {
                    sleep(transaction_sync_interval).await;
                    info!("Updating transactions");

                    // select last seen node as block sync node
                    let sync_node = peers.last_seen();
                    transactions.update(sync_node);
                }
            });
        }

        let server = self.clone();
        let mut receiver = self.inbound.take_receiver();
        task::spawn(async move {
            loop {
                if let Err(e) = server.process_incoming_messages(&mut receiver).await {
                    error!("Server error: {}", e);
                }
            }
        });
    }

    pub async fn start(&mut self) -> Result<(), NetworkError> {
        debug!("Initializing the connection server");
        self.establish_address().await?;
        self.start_services().await;
        debug!("Connection server initialized");

        Ok(())
    }

    #[inline]
    pub fn local_address(&self) -> Option<SocketAddr> {
        self.environment.local_address()
    }

    async fn process_incoming_messages(&self, receiver: &mut Receiver) -> Result<(), NetworkError> {
        let Message { direction, payload } = receiver.recv().await.ok_or(NetworkError::ReceiverFailedToParse)?;

        let source = if let Direction::Inbound(addr) = direction {
            self.peers.update_last_seen(addr);
            Some(addr)
        } else {
            None
        };

        match payload {
            Payload::ConnectingTo(remote_address) => {
                if direction == Direction::Internal {
                    self.peers.connecting_to_peer(remote_address)?;
                }
            }
            Payload::ConnectedTo(remote_address, remote_listener) => {
                if direction == Direction::Internal {
                    self.peers.connected_to_peer(remote_address, remote_listener)?;
                }
            }
            Payload::Transaction(transaction) => {
                let connected_peers = self.peers.connected_peers();
                self.transactions
                    .received_transaction(source.unwrap(), transaction, connected_peers)
                    .await?;
            }
            Payload::Block(block) => {
                self.blocks
                    .received_block(source.unwrap(), block, Some(self.peers.connected_peers()))
                    .await?;
            }
            Payload::SyncBlock(block) => {
                self.blocks.received_block(source.unwrap(), block, None).await?;
            }
            Payload::GetBlock(hash) => {
                self.blocks.received_get_block(source.unwrap(), hash).await?;
            }
            Payload::GetMemoryPool => {
                self.transactions.received_get_memory_pool(source.unwrap()).await?;
            }
            Payload::MemoryPool(mempool) => {
                self.transactions.received_memory_pool(mempool)?;
            }
            Payload::GetSync(getsync) => {
                self.blocks.received_get_sync(source.unwrap(), getsync).await?;
            }
            Payload::Sync(sync) => {
                self.blocks.received_sync(source.unwrap(), sync).await;
            }
            Payload::Disconnect(addr) => {
                if direction == Direction::Internal {
                    self.peers.disconnected_from_peer(addr)?;
                }
            }
            Payload::GetPeers => {
                self.peers.send_get_peers(source.unwrap());
            }
            Payload::Peers(peers) => {
                self.peers.process_inbound_peers(peers);
            }
            Payload::Ping(_block_height) => {
                self.outbound
                    .send_request(Message::new(Direction::Outbound(source.unwrap()), Payload::Pong));
                // TODO(ljedrz/niklas): perform a sync if needed
            }
            Payload::Pong => {
                self.peers.received_pong(source.unwrap());
            }
        }

        Ok(())
    }
}
