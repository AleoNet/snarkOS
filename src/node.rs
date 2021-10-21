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

use crate::{helpers::Tasks, Environment, NodeType, Peers};
use snarkos_ledger::{ledger::Ledger, storage::rocksdb::RocksDB};
use snarkvm::dpc::{Address, Block, Network};

use anyhow::{anyhow, Result};
use rand::{thread_rng, Rng};
use std::{
    marker::PhantomData,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
};
use tokio::{sync::Mutex, task};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Status {
    Idle = 0,
    Mining,
    Syncing,
    ShuttingDown,
}

/// A node server implementation.
#[derive(Clone)]
pub struct Node<E: Environment, N: Network> {
    /// A random numeric identifier for the node.
    id: u64,
    /// The current status of the node.
    status: Arc<AtomicU8>,
    /// The list of peers for the node.
    peers: Arc<Mutex<Peers<N>>>,
    /// The ledger state of the node.
    ledger: Ledger<N>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
    /// A terminator bit for the miner.
    terminator: Arc<AtomicBool>,
    /// Phantom data.
    _phantom: PhantomData<E>,
}

impl<E: Environment, N: Network> Node<E, N> {
    pub fn new() -> Result<Self> {
        // Initialize the node.
        let node = Self {
            id: thread_rng().gen(),
            status: Arc::new(AtomicU8::new(0)),
            peers: Arc::new(Mutex::new(Peers::new())),
            ledger: Ledger::<N>::open::<RocksDB, _>(&format!(".ledger-{}", thread_rng().gen::<u8>()))?,
            tasks: Tasks::new(),
            terminator: Arc::new(AtomicBool::new(false)),
            _phantom: PhantomData,
        };
        Ok(node)
    }

    /// Returns the current status of the node.
    #[inline]
    pub fn status(&self) -> Status {
        match self.status.load(Ordering::SeqCst) {
            0 => Status::Idle,
            1 => Status::Mining,
            2 => Status::Syncing,
            3 => Status::ShuttingDown,
            _ => unreachable!("Invalid status code"),
        }
    }

    /// Updates the node to the given status.
    #[inline]
    pub fn set_status(&self, state: Status) {
        self.status.store(state as u8, Ordering::SeqCst);
        match state {
            Status::ShuttingDown => {
                // debug!("Shutting down");
                self.terminator.store(true, Ordering::SeqCst);
                self.tasks.flush();
            }
            _ => (),
        }
    }

    /// Initializes the listener for peers.
    #[inline]
    pub async fn start_listener(&self, port: u16) -> Result<()> {
        let peers = self.peers.clone();
        let listener = Peers::listen::<E>(peers, port).await?;
        self.add_task(listener)
    }

    /// Initializes a miner.
    #[inline]
    pub fn start_miner(&self, miner_address: Address<N>) -> Result<()> {
        // If the node is a mining node, initialize a miner.
        if E::NODE_TYPE != NodeType::Miner {
            Err(anyhow!("Node is not a mining node"))
        } else {
            let node = self.clone();
            self.add_task(task::spawn(async move {
                let rng = &mut thread_rng();
                let mut ledger = node.ledger.clone();
                loop {
                    // Retrieve the status of the node.
                    let status = node.status();
                    // Ensure the node is not syncing or shutting down.
                    if status != Status::Syncing && status != Status::ShuttingDown {
                        // Set the status of the node to mining.
                        node.set_status(Status::Mining);
                        // Start the mining process.
                        let miner = ledger.mine_next_block(miner_address, &node.terminator, rng);
                        // Ensure the miner did not error.
                        if let Err(error) = miner {
                            error!("{}", error);
                        }
                    }
                }
            }))
        }
    }

    /// Initializes the peers.
    #[inline]
    pub async fn connect_to(&self, remote_ip: SocketAddr) {
        debug!("Connecting to {}...", remote_ip);
        if let Err(error) = Peers::connect_to::<E>(self.peers.clone(), remote_ip).await {
            error!("{}", error)
        }
    }

    /// Adds the given task handle to the node.
    #[inline]
    pub fn add_task(&self, handle: task::JoinHandle<()>) -> Result<()> {
        self.tasks.append(handle);
        Ok(())
    }

    // /// Returns a version message for this node.
    // #[inline]
    // pub fn version(&self) -> Version {
    //     Version::new(E::PROTOCOL_VERSION, self.expect_local_addr().port(), self.id)
    // }
    //
    // #[deprecated]
    // #[inline]
    // pub fn expect_local_addr(&self) -> SocketAddr {
    //     self.local_ip.get().copied().expect("no address set!")
    // }

    /// Disconnects from peers and proceeds to shut down the node.
    #[inline]
    pub async fn shut_down(&self) {
        self.set_status(Status::ShuttingDown);
        // for address in self.connected_peers() {
        //     self.disconnect_from_peer(address).await;
        // }
    }
}
