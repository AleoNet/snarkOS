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

use crate::{environment::Environment, Data, Message, OutboundRouter};

use snarkvm::prelude::*;

use indexmap::IndexMap;
use std::{
    marker::PhantomData,
    net::SocketAddr,
    sync::Arc,
    time::{Instant, SystemTime},
};
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    task,
};

/// Shorthand for the parent half of the `Peers` message channel.
pub(crate) type PeersRouter<N> = mpsc::Sender<PeersRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Peers` message channel.
type PeersHandler<N> = mpsc::Receiver<PeersRequest<N>>;

///
/// An enum of requests that the `Peers` struct processes.
///
#[derive(Debug)]
pub enum PeersRequest<N: Network> {
    /// Heartbeat
    Heartbeat,
    /// MessagePropagate := (peer_ip, message)
    MessagePropagate(SocketAddr, Message<N>),
    /// MessageSend := (peer_ip, message)
    MessageSend(SocketAddr, Message<N>),
    /// PeerConnected := (peer_ip, outbound_router)
    PeerConnected(SocketAddr, OutboundRouter<N>),
    /// PeerDisconnected := (peer_ip)
    PeerDisconnected(SocketAddr),
    /// PeerRestricted := (peer_ip)
    PeerRestricted(SocketAddr),
}

///
/// A list of peers connected to the node server.
///
pub struct Peers<N: Network, E: Environment> {
    /// The peers router of the node.
    peers_router: PeersRouter<N>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The map connected peer IPs to their nonce and outbound message router.
    connected_peers: RwLock<IndexMap<SocketAddr, OutboundRouter<N>>>,
    /// The set of restricted peer IPs.
    restricted_peers: RwLock<IndexMap<SocketAddr, Instant>>,
    /// The map of peers to their first-seen port number, number of attempts, and timestamp of the last inbound connection request.
    seen_inbound_connections: RwLock<IndexMap<SocketAddr, ((u16, u32), SystemTime)>>,
    /// The map of peers to the timestamp of their last outbound connection request.
    seen_outbound_connections: RwLock<IndexMap<SocketAddr, SystemTime>>,
    _phantom: PhantomData<E>,
}

impl<N: Network, E: Environment> Peers<N, E> {
    ///
    /// Initializes a new instance of `Peers`.
    ///
    pub async fn new(local_ip: SocketAddr) -> Arc<Self> {
        // Initialize an mpsc channel for sending requests to the `Peers` struct.
        let (peers_router, mut peers_handler) = mpsc::channel(1024);

        // Initialize the peers.
        let peers = Arc::new(Self {
            peers_router,
            local_ip,
            connected_peers: Default::default(),
            restricted_peers: Default::default(),
            seen_inbound_connections: Default::default(),
            seen_outbound_connections: Default::default(),
            _phantom: PhantomData,
        });

        // Initialize the peers router process.
        {
            let peers = peers.clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None,
                task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // Asynchronously wait for a peers request.
                    while let Some(request) = peers_handler.recv().await {
                        let peers = peers.clone();
                        // Asynchronously process a peers request.
                        E::resources().register_task(
                            None,
                            task::spawn(async move {
                                // Hold the peers write lock briefly, to update the state of the peers.
                                peers.update(request).await;
                            }),
                        );
                    }
                }),
            );
            // Wait until the peers router task is ready.
            let _ = handler.await;
        }

        peers
    }

    /// Returns an instance of the peers router.
    pub fn router(&self) -> PeersRouter<N> {
        self.peers_router.clone()
    }

    /// Returns the IP address of this node.
    pub const fn local_ip(&self) -> &SocketAddr {
        &self.local_ip
    }

    ///
    /// Returns `true` if the node is connected to the given IP.
    ///
    pub async fn is_connected_to(&self, ip: &SocketAddr) -> bool {
        self.connected_peers.read().await.contains_key(ip)
    }

    ///
    /// Returns `true` if the given IP is restricted.
    ///
    pub async fn is_restricted(&self, ip: SocketAddr) -> bool {
        match self.restricted_peers.read().await.get(&ip) {
            Some(timestamp) => timestamp.elapsed().as_secs() < E::RADIO_SILENCE_IN_SECS,
            None => false,
        }
    }

    ///
    /// Returns the list of connected peers.
    ///
    pub async fn connected_peers(&self) -> Vec<SocketAddr> {
        self.connected_peers.read().await.keys().copied().collect()
    }

    ///
    /// Returns the number of connected peers.
    ///
    pub async fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().await.len()
    }

    ///
    /// Performs the given `request` to the peers.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: PeersRequest<N>) {
        match request {
            PeersRequest::Heartbeat => {
                // TODO (raychu86): Implement this.
            }
            PeersRequest::MessagePropagate(sender, message) => {
                self.propagate(sender, message).await;
            }
            PeersRequest::MessageSend(sender, message) => {
                self.send(sender, message).await;
            }
            PeersRequest::PeerConnected(peer_ip, outbound) => {
                // Add an entry for this `Peer` in the connected peers.
                self.connected_peers.write().await.insert(peer_ip, outbound);
            }
            PeersRequest::PeerDisconnected(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
            }
            PeersRequest::PeerRestricted(peer_ip) => {
                // Remove an entry for this `Peer` in the connected peers, if it exists.
                self.connected_peers.write().await.remove(&peer_ip);
                // Add an entry for this `Peer` in the restricted peers.
                self.restricted_peers.write().await.insert(peer_ip, Instant::now());
            }
        }
    }

    ///
    /// Sends the given message to specified peer.
    ///
    async fn send(&self, peer: SocketAddr, message: Message<N>) {
        let target_peer = self.connected_peers.read().await.get(&peer).cloned();
        match target_peer {
            Some(outbound) => {
                if let Err(error) = outbound.send(message).await {
                    trace!("Outbound channel failed: {}", error);
                    self.connected_peers.write().await.remove(&peer);
                }
            }
            None => warn!("Attempted to send to a non-connected peer {}", peer),
        }
    }

    ///
    /// Sends the given message to every connected peer, excluding the sender.
    ///
    async fn propagate(&self, sender: SocketAddr, mut message: Message<N>) {
        // Perform ahead-of-time, non-blocking serialization just once for applicable objects.
        if let Message::BlockBroadcast(ref mut data) = message {
            let serialized_block = Data::serialize(data.clone()).await.expect("Block serialization is bugged");
            let _ = std::mem::replace(data, Data::Buffer(serialized_block));
        }

        // Iterate through all peers that are not the sender, sync node, or beacon node.
        for peer in self
            .connected_peers()
            .await
            .iter()
            .filter(|peer_ip| *peer_ip != &sender && !E::beacon_nodes().contains(peer_ip))
            .copied()
            .collect::<Vec<_>>()
        {
            self.send(peer, message.clone()).await;
        }
    }

    ///
    /// Removes the addresses of all known peers.
    ///
    #[cfg(feature = "test")]
    pub async fn reset_known_peers(&self) {
        self.restricted_peers.write().await.clear();
        self.seen_inbound_connections.write().await.clear();
        self.seen_outbound_connections.write().await.clear();
    }
}
