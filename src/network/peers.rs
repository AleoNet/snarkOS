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

// use mpmc_map::MpmcMap;
// use std::{
//     net::SocketAddr,
//     sync::{atomic::AtomicU32, Arc},
// };
//
// #[derive(Debug)]
// pub enum PeerEventData {
//     Connected(PeerHandle),
//     Disconnect(Box<Peer>),
//     FailHandshake,
// }
//
// #[derive(Debug)]
// pub struct PeerEvent {
//     pub address: SocketAddr,
//     pub data: PeerEventData,
// }
//
// /// A data structure containing information about a peer.
// #[derive(Debug, Clone)]
// pub struct Peer {
//     /// The address of the node's listener socket.
//     pub address: SocketAddr,
//     /// The latest broadcast block height of the peer.
//     pub block_height: u32,
//     /// Quantifies the node's connection quality with the peer.
//     pub quality: PeerQuality,
//     /// Tracks the node's sync state with the peer.
//     pub sync_state: SyncState,
//
//     /// The cache of received blocks from the peer.
//     pub block_received_cache: BlockCache<{ crate::PEER_BLOCK_CACHE_SIZE }>,
// }
//
// ///
// /// A data structure for storing the peers of this node server.
// ///
// pub struct Peers {
//     /// The list of connected peers.
//     connected_peers: MpmcMap<SocketAddr, PeerHandle>,
//     /// The list of disconnected peers.
//     disconnected_peers: MpmcMap<SocketAddr, Peer>,
//
//     pending_connections: Arc<AtomicU32>,
//     peer_events: wrapped_mpsc::Sender<PeerEvent>,
// }
