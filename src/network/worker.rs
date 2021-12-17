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
    NodeType,
    PeersRouter,
};

use snarkos_storage::{storage::Storage, BlockTemplate};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    net::SocketAddr,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::{
    sync::{mpsc, oneshot},
    task,
    task::JoinHandle,
};

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
    /// The thread pool for the worker.
    worker: Arc<ThreadPool>,
    /// The address of the worker.
    worker_address: Option<Address<N>>,
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
        // Initialize an mpsc channel for sending requests for the `Worker` struct.
        let (worker_router, mut worker_handler) = mpsc::channel(1024);
        // Initialize the worker pool.
        let pool = ThreadPoolBuilder::new()
            .stack_size(8 * 1024 * 1024)
            .num_threads((num_cpus::get() / 8 * 7).max(1))
            .build()?;

        // Initialize the worker.
        let worker = Arc::new(Self {
            worker: Arc::new(pool),
            worker_address: miner,
            worker_router,
            status: status.clone(),
            terminator: terminator.clone(),
            peers_router,
            ledger_reader,
            ledger_router,
            pool_address,
        });

        // Initialize the handler for the worker.
        {
            let worker = worker.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a worker request.
                while let Some(request) = worker_handler.recv().await {
                    worker.update(request).await;
                }
            }));
            // Wait until the worker handler is ready.
            let _ = handler.await;
        }

        if E::NODE_TYPE == NodeType::Worker {}

        Ok(worker)
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
