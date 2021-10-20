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

use crate::{helpers::Tasks, Environment, NodeType};
use snarkos_ledger::{ledger::Ledger, storage::rocksdb::RocksDB};
use snarkvm::dpc::{Address, Block, Network};

use anyhow::Result;
use once_cell::sync::OnceCell;
use rand::thread_rng;
use std::{
    marker::PhantomData,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
};
use tokio::task;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Status {
    Idle = 0,
    Mining,
    Syncing,
    ShuttingDown,
}

/// A node server implementation.
#[derive(Debug)]
pub struct Node<N: Network, E: Environment<N>> {
    /// The current status of the node.
    status: AtomicU8,
    /// The ledger state of the node.
    ledger: Ledger<N>,
    /// The local address of this node.
    local_addr: OnceCell<SocketAddr>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
    /// A terminator bit for the miner.
    terminator: Arc<AtomicBool>,
    /// Phantom data.
    _phantom: PhantomData<E>,
}

impl<N: Network, E: Environment<N>> Node<N, E> {
    pub async fn new(miner_address: Address<N>) -> Result<Self> {
        // Initialize the node.
        let node = Self {
            status: AtomicU8::new(0),
            ledger: Ledger::<N>::open::<RocksDB, _>(".ledger")?,
            local_addr: OnceCell::new(),
            tasks: Tasks::new(),
            terminator: Arc::new(AtomicBool::new(false)),
            _phantom: PhantomData,
        };

        // If the node is a mining node, initialize a miner.
        if E::NODE_TYPE == NodeType::Miner {
            let mut ledger = node.ledger.clone();
            let terminator = node.terminator.clone();
            node.add_task(task::spawn(async move {
                loop {
                    if let Err(error) = ledger.mine_next_block(miner_address, &terminator, &mut thread_rng()) {
                        error!("{}", error);
                    }
                }
            }));
        }

        Ok(node)
    }

    /// Adds the given task handle to the node.
    #[inline]
    pub fn add_task(&self, handle: task::JoinHandle<()>) {
        self.tasks.append(handle);
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

    /// Disconnects from peers and proceeds to shut down the node.
    #[inline]
    pub async fn shut_down(&self) {
        self.set_status(Status::ShuttingDown);
        // for address in self.connected_peers() {
        //     self.disconnect_from_peer(address).await;
        // }
    }
}
