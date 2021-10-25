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

use crate::{helpers::Tasks, network::server::Server, Environment};
use snarkvm::dpc::{Address, Network};

use anyhow::{anyhow, Result};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use tokio::{runtime, task};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum Status {
    Idle = 0,
    Mining,
    Syncing,
    ShuttingDown,
}

/// A node server implementation.
// #[derive(Clone)]
pub struct Node<N: Network, E: Environment> {
    /// The current status of the node.
    status: Arc<AtomicU8>,
    /// The server of the node.
    server: Server<N, E>,
    /// The list of tasks spawned by the node.
    tasks: Tasks<task::JoinHandle<()>>,
}

impl<N: Network, E: Environment> Node<N, E> {
    pub async fn new(port: u16, miner: Option<Address<N>>) -> Result<Self> {
        // Initialize the node.
        let node = Self {
            status: Arc::new(AtomicU8::new(0)),
            server: Server::initialize(port, miner).await?,
            tasks: Tasks::new(),
        };
        Ok(node)
    }

    ///
    /// Returns the current status of the node.
    ///
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
    pub(crate) fn set_status(&self, state: Status) {
        self.status.store(state as u8, Ordering::SeqCst);
        match state {
            Status::ShuttingDown => {
                // debug!("Shutting down");
                // self.terminator.store(true, Ordering::SeqCst);
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
