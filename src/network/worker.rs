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

use crate::{
    helpers::{Status, Tasks},
    Environment,
    LedgerReader,
    LedgerRouter,
    PeersRouter,
};

use snarkos_storage::{storage::Storage, BlockTemplate};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
use std::{
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::{sync::mpsc, task::JoinHandle};

/// Shorthand for the parent half of the `Worker` message channel.
pub(crate) type WorkerRouter<N> = mpsc::Sender<WorkerRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Worker` message channel.
type WorkerHandler<N> = mpsc::Receiver<WorkerRequest<N>>;

///
/// An enum of requests that the `Worker` struct processes.
///
#[derive(Debug)]
pub enum WorkerRequest<N: Network> {
    /// BlockTemplate := (peer_ip, share_difficulty, block_template)
    BlockTemplate(SocketAddr, u64, BlockTemplate<N>),
}

///
/// A pool worker for a specific network on the node server.
///
#[derive(Debug)]
pub struct Worker<N: Network, E: Environment> {
    /// The address of the worker.
    worker_address: Option<Address<N>>,
    /// The local address of the worker.
    local_ip: SocketAddr,
    /// The worker router of the node.
    worker_router: WorkerRouter<N>,
    /// The status of the node.
    status: Status,
    /// A terminator bit for the worker.
    terminator: Arc<AtomicBool>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The ledger state of the node.
    ledger_reader: LedgerReader<N>,
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N>,
    /// The address of the connected pool.
    pool_address: Option<SocketAddr>,
}

impl<N: Network, E: Environment> Worker<N, E> {
    /// Initializes a new instance of the worker.
    pub async fn open<S: Storage>(
        tasks: &mut Tasks<JoinHandle<()>>,
        miner: Option<Address<N>>,
        local_ip: SocketAddr,
        status: &Status,
        terminator: &Arc<AtomicBool>,
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        pool_address: Option<SocketAddr>,
    ) -> Result<Arc<Self>> {
    }

    /// Returns an instance of the worker router.
    pub fn router(&self) -> WorkerRouter<N> {
        self.worker_router.clone()
    }

    ///
    /// Performs the given `request` to the worker.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: WorkerRequest<N>) {
        match request {
            WorkerRequest::BlockTemplate(_peer_ip, share_difficulty, block_template) => {}
        }
    }
}
