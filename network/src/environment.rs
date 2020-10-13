// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::{external::PingPongManager, NetworkError, PeerManager, ReceiveHandler, SendHandler, SyncManager};
use snarkos_consensus::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_objects::Network;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::{
    runtime::Runtime,
    sync::{Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// TODO (howardwu): Remove pub from each field and add getters only.
/// A core data structure containing the networking parameters for this node.
#[derive(Clone)]
pub struct Environment {
    /// TODO (howardwu): Rearchitect the ledger to be thread safe with shared ownership.
    /// The storage system of this node.
    storage: Arc<RwLock<MerkleTreeLedger>>,
    /// The memory pool of this node.
    memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
    /// The consensus parameters for the associated network ID.
    consensus_parameters: Arc<ConsensusParameters>,
    /// The DPC parameters for the associated network ID.
    dpc_parameters: Arc<PublicParameters<Components>>,
    /// The network ID of this node.
    network_id: Network,
    /// The send handler of this node.
    send_handler: SendHandler,
    /// The receive handler of this node.
    receive_handler: ReceiveHandler,

    /// TODO (howardwu): Remove this.
    /// The ping pong manager of this node.
    /// Ping/pongs with connected peers
    ping_pong: Arc<RwLock<PingPongManager>>,
    // /// TODO (howardwu): Remove this.
    // /// The handshakes with connected peers
    // handshakes: Arc<RwLock<HashMap<SocketAddr, Handshake>>>,
    /// TODO (howardwu): Remove this.
    pub(crate) peer_manager: Option<Arc<RwLock<PeerManager>>>,
    /// TODO (howardwu): Remove this.
    sync_manager: Option<Arc<Mutex<SyncManager>>>,

    /// The local address of this node.
    local_address: SocketAddr,

    /// The minimum number of peers required to maintain connections with.
    minimum_number_of_peers: u16,
    /// The maximum number of peers permitted to maintain connections with.
    maximum_number_of_peers: u16,

    /// TODO (howardwu): Rename CONNECTION_FREQUENCY to this.
    /// The number of milliseconds this node waits to perform a periodic sync with its peers.
    sync_interval: u64,
    /// TODO (howardwu): this is not in seconds. deprecate this and rearchitect it.
    /// The number of seconds this node waits to request memory pool transactions from its peers.
    memory_pool_interval: u8,

    /// The default bootnodes of the network.
    bootnodes: Vec<SocketAddr>,
    /// If `true`, initializes this node as a bootnode and forgoes connecting
    /// to the default bootnodes or saved peers in the peer book.
    is_bootnode: bool,
    /// If `true`, initializes a mining thread on this node.
    is_miner: bool,
}

impl Environment {
    /// Creates a new instance of `Environment`.
    #[inline]
    pub fn new(
        storage: Arc<RwLock<MerkleTreeLedger>>,
        memory_pool: Arc<Mutex<MemoryPool<Tx>>>,
        consensus_parameters: Arc<ConsensusParameters>,
        dpc_parameters: Arc<PublicParameters<Components>>,

        local_address: SocketAddr,

        min_peers: u16,
        max_peers: u16,
        sync_interval: u64,
        memory_pool_interval: u8,

        bootnodes_addresses: Vec<String>,
        is_bootnode: bool,
        is_miner: bool,
    ) -> Result<Self, NetworkError> {
        // Check that the minimum and maximum number of peers is valid.
        if min_peers == 0 || max_peers == 0 {
            return Err(NetworkError::PeerCountInvalid);
        }

        // Check that the sync interval is a reasonable number of seconds.
        if sync_interval < 2 || sync_interval > 300 {
            return Err(NetworkError::SyncIntervalInvalid);
        }

        // TODO (howardwu): Check the memory pool interval.

        // Convert the given bootnodes into socket addresses.
        let mut bootnodes = Vec::with_capacity(bootnodes_addresses.len());
        for bootnode_address in bootnodes_addresses.iter() {
            if let Ok(bootnode) = bootnode_address.parse::<SocketAddr>() {
                bootnodes.push(bootnode);
            }
        }

        // Derive the network ID.
        let network_id = consensus_parameters.network_id;

        // Create a new send handler.
        let send_handler = SendHandler::new();

        // Create a new receive handler.
        let receive_handler = ReceiveHandler::new();

        // TODO (howardwu): Remove this.
        // Create the ping pong manager.
        let ping_pong = Arc::new(RwLock::new(PingPongManager::new()));
        // TODO (howardwu): Remove this.
        // Create a new handshake struct.
        // let handshakes = Arc::new(RwLock::new(HashMap::default()));

        Ok(Self {
            storage,
            memory_pool,
            consensus_parameters,
            dpc_parameters,
            network_id,
            send_handler,
            receive_handler,

            ping_pong,
            // handshakes,
            peer_manager: None, // TODO (howardwu): Remove this
            sync_manager: None, // TODO (howardwu): Remove this

            local_address,

            minimum_number_of_peers: min_peers,
            maximum_number_of_peers: max_peers,
            sync_interval,
            memory_pool_interval,

            bootnodes,
            is_bootnode,
            is_miner,
        })
    }

    /// TODO (howardwu): Remove this.
    pub fn set_managers(&mut self) {
        // TODO (howardwu): Remove this.
        let peer_manager = Runtime::new()
            .unwrap()
            .block_on(PeerManager::new(self.clone()))
            .unwrap();
        self.peer_manager = Some(Arc::new(RwLock::new(peer_manager)));

        self.sync_manager = Some(Arc::new(Mutex::new(SyncManager::new(
            self.clone(),
            *self.bootnodes.first().unwrap(),
        ))));
    }

    // /// TODO (howardwu): Remove this.
    // #[inline]
    // pub async fn peer_manager(&self) -> &Option<Arc<RwLock<PeerManager>>> {
    //     &self.peer_manager
    // }

    /// TODO (howardwu): Remove this.
    #[inline]
    pub async fn peer_manager_read(&self) -> RwLockReadGuard<'_, PeerManager> {
        self.peer_manager.as_ref().unwrap().read().await
    }

    /// TODO (howardwu): Remove this.
    #[inline]
    pub async fn peer_manager_write(&self) -> RwLockWriteGuard<'_, PeerManager> {
        self.peer_manager.as_ref().unwrap().write().await
    }

    /// TODO (howardwu): Remove this.
    #[inline]
    pub async fn sync_manager(&self) -> &Arc<Mutex<SyncManager>> {
        self.sync_manager.as_ref().unwrap()
    }

    // /// TODO (howardwu): Remove this.
    // #[inline]
    // pub async fn peer_manager_read(&self) -> RwLockReadGuard<'_, PeerManager> {
    //     let peer_manager = self.peer_manager.unwrap().clone();
    //     peer_manager.read().await
    // }
    //
    // /// TODO (howardwu): Remove this.
    // #[inline]
    // pub async fn peer_manager_write(&self) -> RwLockWriteGuard<'_, PeerManager> {
    //     let peer_manager = self.peer_manager.unwrap().clone();
    //     peer_manager.unwrap().write().await
    // }

    /// Returns a reference to the storage system of this node.
    #[inline]
    pub fn storage(&self) -> &Arc<RwLock<MerkleTreeLedger>> {
        &self.storage
    }

    /// Returns a reference to the memory pool of this node.
    #[inline]
    pub fn memory_pool(&self) -> &Arc<Mutex<MemoryPool<Tx>>> {
        &self.memory_pool
    }

    /// Returns a reference to the consensus parameters of this node.
    #[inline]
    pub fn consensus_parameters(&self) -> &Arc<ConsensusParameters> {
        &self.consensus_parameters
    }

    /// Returns a reference to the DPC parameters of this node.
    #[inline]
    pub fn dpc_parameters(&self) -> &Arc<PublicParameters<Components>> {
        &self.dpc_parameters
    }

    /// Returns a reference to the send handler of this node.
    #[inline]
    pub fn send_handler(&self) -> &SendHandler {
        &self.send_handler
    }

    /// Returns a reference to the receive handler of this node.
    #[inline]
    pub fn receive_handler(&self) -> &ReceiveHandler {
        &self.receive_handler
    }

    /// Returns a reference to the ping pong manager of this node.
    #[inline]
    pub fn ping_pong(&self) -> &Arc<RwLock<PingPongManager>> {
        &self.ping_pong
    }

    /// Returns a reference to the handshakes of this node.
    // #[inline]
    // pub fn handshakes(&self) -> &Arc<RwLock<HashMap<SocketAddr, Handshake>>> {
    //     &self.handshakes
    // }

    /// Returns a reference to the default bootnodes of the network.
    #[inline]
    pub fn local_address(&self) -> &SocketAddr {
        &self.local_address
    }

    /// Returns a reference to the default bootnodes of the network.
    #[inline]
    pub fn bootnodes(&self) -> &Vec<SocketAddr> {
        &self.bootnodes
    }

    /// Returns `true` if this node is a bootnode. Otherwise, returns `false`.
    #[inline]
    pub fn is_bootnode(&self) -> bool {
        self.is_bootnode
    }

    /// Returns `true` if this node is a mining node. Otherwise, returns `false`.
    #[inline]
    pub fn is_miner(&self) -> bool {
        self.is_miner
    }

    /// Returns the minimum number of peers this node maintains a connection with.
    #[inline]
    pub fn minimum_number_of_peers(&self) -> u16 {
        self.minimum_number_of_peers
    }

    /// Returns the maximum number of peers this node maintains a connection with.
    #[inline]
    pub fn maximum_number_of_peers(&self) -> u16 {
        self.minimum_number_of_peers
    }

    /// Returns the sync interval of this node.
    #[inline]
    pub fn sync_interval(&self) -> u64 {
        self.sync_interval
    }

    /// Returns the memory pool interval of this node.
    #[inline]
    pub fn memory_pool_interval(&self) -> u8 {
        self.memory_pool_interval
    }

    /// Returns the current block height of the ledger from storage.
    #[inline]
    pub async fn current_block_height(&self) -> u32 {
        self.storage.read().await.get_current_block_height()
    }

    /// Attempts to acquire a read lock for storage.
    #[inline]
    pub async fn storage_read(&self) -> RwLockReadGuard<'_, MerkleTreeLedger> {
        self.storage.read().await
    }

    /// Attempts to acquire the write lock for storage.
    #[inline]
    pub async fn storage_mut(&self) -> RwLockWriteGuard<'_, MerkleTreeLedger> {
        self.storage.write().await
    }
}
