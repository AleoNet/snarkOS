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

// Compilation
#![warn(unused_extern_crates)]
#![forbid(unsafe_code)]
// Documentation
#![cfg_attr(nightly, feature(doc_cfg, external_doc))]
#![cfg_attr(nightly, doc(include = "../documentation/concepts/network_server.md"))]

#[macro_use]
extern crate log;
#[macro_use]
extern crate snarkos_metrics;

pub mod external;
pub mod peers;

pub mod environment;
pub use environment::*;

pub mod errors;
pub use errors::*;

pub mod inbound;
pub use inbound::*;

pub mod outbound;
pub use outbound::*;

pub mod manager;
pub use manager::*;

pub mod sync_manager;
pub use sync_manager::*;

use crate::{
    environment::Environment,
    inbound::Response,
    manager::PeerManager,
    Inbound,
    NetworkError,
    Outbound,
    SyncManager,
};
use snarkos_errors::{
    consensus::ConsensusError,
    network::{ConnectError, SendError},
    objects::BlockError,
    storage::StorageError,
};

use std::{fmt, net::Shutdown, time::Duration};
use tokio::time::sleep;

/// A core data structure for operating the networking stack of this node.
pub struct Server {
    peer_manager: PeerManager,
    // sync_manager: Arc<Mutex<SyncManager>>,
}

impl Server {
    /// Creates a new instance of `Server`.
    pub async fn new(environment: &mut Environment) -> Result<Self, NetworkError> {
        let peer_manager = PeerManager::new(&mut environment.clone())?;
        environment.set_managers();
        Ok(Self { peer_manager })
    }

    ///
    /// Starts the server event loop.
    ///
    /// 1. Initialize TCP listener at `local_address` and accept new TCP connections.
    /// 2. Spawn a new thread to handle new connections.
    /// 3. Start the connection handler.
    /// 4. Start the message handler.
    ///
    pub async fn listen(self) -> Result<(), NetworkError> {
        self.peer_manager.initialize().await?;
        loop {
            info!("Hello b?");
            self.peer_manager.clone().update().await?;
            sleep(Duration::from_secs(10)).await;
        }
    }
}
