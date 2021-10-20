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

use crate::{network::peer::Peer, Environment, Node};
use snarkvm::dpc::Network;

use anyhow::Result;
use mpmc_map::MpmcMap;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};
use tokio::net::TcpStream;

///
/// A data structure for storing the peers of this node server.
///
#[derive(Clone, Debug)]
pub struct Peers<N: Network, E: Environment<N>> {
    /// The list of connected peers.
    connected_peers: MpmcMap<SocketAddr, Peer<N, E>>,
    /// The list of disconnected peers.
    disconnected_peers: MpmcMap<SocketAddr, Peer<N, E>>,
    /// The current number of pending connections.
    pending_connections: Arc<AtomicU32>,
}

impl<N: Network, E: Environment<N>> Peers<N, E> {
    pub fn new() -> Self {
        Self {
            connected_peers: Default::default(),
            disconnected_peers: Default::default(),
            pending_connections: Default::default(),
        }
    }

    pub async fn connect_to(&self, node: Node<N, E>, address: SocketAddr) -> Result<()> {
        if let Some(_) = self.connected_peers.get(&address) {
            Ok(())
        } else {
            // if let Some(mut peer) = self.disconnected_peers.get(&address) {
            //     if peer.judge_bad_offline() {
            //         // dont reconnect to bad peers
            //         return Ok(None);
            //     }
            // }
            let peer = if let Some(mut peer) = self.disconnected_peers.remove(address).await {
                peer
            } else {
                Peer::new(address)
            };
            self.pending_connections.fetch_add(1, Ordering::SeqCst);
            peer.connect(node);
            Ok(())
        }
    }

    pub fn receive_connection(&self, node: Node<N, E>, remote_ip: SocketAddr, stream: TcpStream) -> Result<()> {
        self.pending_connections.fetch_add(1, Ordering::SeqCst);
        Peer::receive(remote_ip, node, stream);
        Ok(())
    }

    pub async fn fetch_received_peer_data(&self, ip: SocketAddr) -> Peer<N, E> {
        if let Some(peer) = self.disconnected_peers.remove(ip).await {
            peer
        } else {
            Peer::new(ip)
        }
    }
}
