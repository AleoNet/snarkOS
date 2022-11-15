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

use snarkos_node_executor::{NodeType, RawStatus};
use snarkos_node_network::{ConnectionSide, Network};

use parking_lot::RwLock;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::Arc,
};
use time::OffsetDateTime;

#[derive(Clone)]
pub struct Router {
    network: Network,
    connection_meta: Arc<RwLock<HashMap<SocketAddr, ConnectionMeta>>>,
    trusted_peers: Arc<HashSet<SocketAddr>>,
    candidate_peers: Arc<RwLock<HashSet<SocketAddr>>>,
    restricted_peers: Arc<RwLock<HashMap<SocketAddr, OffsetDateTime>>>,
}

impl Router {
    pub async fn new() -> Self {
        Self {
            network: Network::new(Default::default()).await.unwrap(),
            connection_meta: Default::default(),
            trusted_peers: Default::default(),
            candidate_peers: Default::default(),
            restricted_peers: Default::default(),
        }
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn trusted_peers(&self) -> &HashSet<SocketAddr> {
        &self.trusted_peers
    }

    pub fn candidate_peers(&self) -> Vec<SocketAddr> {
        self.candidate_peers.read().iter().copied().collect()
    }

    pub fn insert_candidate_peer(&self, addr: SocketAddr) {
        self.candidate_peers.write().insert(addr);
    }

    pub fn remove_candidate_peer(&self, addr: SocketAddr) {
        self.candidate_peers.write().remove(&addr);
    }

    pub fn insert_restricted_peer(&self, addr: SocketAddr) {
        self.restricted_peers.write().insert(addr, OffsetDateTime::now_utc());
    }

    pub fn remove_restricted_peer(&self, addr: SocketAddr) {
        self.restricted_peers.write().remove(&addr);
    }

    pub fn connected_beacons(&self) -> Vec<SocketAddr> {
        self.connection_meta
            .read()
            .iter()
            .filter(|(_addr, meta)| meta.node_type == NodeType::Beacon)
            .map(|(addr, _meta)| addr)
            .copied()
            .collect()
    }
}

// TODO(nkls): split into separate module

#[derive(Debug, Clone)]
pub struct ConnectionMeta {
    side: ConnectionSide,
    listening_addr: SocketAddr,

    // TODO(nkls): potentially split this out.
    // Peer Meta:
    version: u32,
    node_type: NodeType,
    status: RawStatus,
    block_height: Arc<RwLock<u32>>, // TODO(nkls): this could probably be an atomic.
    last_seen: Arc<RwLock<OffsetDateTime>>,
    seen_messages: Arc<RwLock<HashMap<(u16, u32), OffsetDateTime>>>,
}

impl ConnectionMeta {
    pub fn new(
        side: ConnectionSide,
        listening_addr: SocketAddr,
        version: u32,
        node_type: NodeType,
        status: RawStatus,
    ) -> Self {
        Self {
            side,
            listening_addr,
            version,
            node_type,
            status,
            block_height: Arc::new(RwLock::new(0)),
            last_seen: Arc::new(RwLock::new(OffsetDateTime::now_utc())),
            seen_messages: Default::default(),
        }
    }
}
