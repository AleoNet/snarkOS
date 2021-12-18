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
    helpers::{State, Status, Tasks},
    Data,
    Environment,
    LedgerReader,
    Message,
    NodeType,
    PeersRequest,
    PeersRouter,
};

use snarkos_storage::storage::Storage;
use snarkvm::dpc::prelude::*;

use anyhow::{anyhow, Result};
use rand::thread_rng;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
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
    /// The address of the connected pool.
    pool_address: Option<SocketAddr>,
}

impl<N: Network, E: Environment> Worker<N, E> {
    /// Initializes a new instance of the worker.
    pub async fn open<S: Storage>(
        tasks: &mut Tasks<JoinHandle<()>>,
        miner: Option<Address<N>>,
        status: &Status,
        terminator: &Arc<AtomicBool>,
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
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

        if E::NODE_TYPE == NodeType::Worker {
            if let Some(pool_address) = pool_address {
                if let Some(recipient) = miner {
                    // Ask for our first block template to get the loop started.
                    if let Err(error) = worker
                        .peers_router
                        .send(PeersRequest::MessageSend(pool_address, Message::GetWork(recipient)))
                        .await
                    {
                        warn!("Could not get block template {}", error);
                    }
                }
            }
        }

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
            WorkerRequest::BlockTemplate(peer_ip, share_difficulty, block_template) => {
                if let Some(pool_address) = self.pool_address {
                    // Refuse work from any pool other than our registered one.
                    if pool_address != peer_ip {
                        return;
                    }

                    if let Some(recipient) = self.worker_address {
                        // Mine the next block continuously until halting. We need to keep going because
                        // a valid block does not necessarily mean we hit the network difficulty target,
                        // only our share target.
                        loop {
                            // Check if we need to halt.
                            let current_height = self.ledger_reader.latest_block_height();
                            if current_height != block_template.block_height() - 1 {
                                // If so, let's ask for a new block template first.
                                if let Err(error) = self
                                    .peers_router
                                    .send(PeersRequest::MessageSend(peer_ip, Message::GetWork(recipient)))
                                    .await
                                {
                                    warn!("Could not send GetWork {}", error);
                                }
                                break;
                            }

                            // If `terminator` is `false` and the status is not `Peering` or `Mining`
                            // already, mine the next block.
                            if !self.terminator.load(Ordering::SeqCst) && !self.status.is_peering() && !self.status.is_mining() {
                                // Set the status to `Mining`.
                                self.status.update(State::Mining);
                                let worker = self.worker.clone();
                                let mut block_template = block_template.clone();
                                let terminator = self.terminator.clone();
                                let peers_router = self.peers_router.clone();
                                let status = self.status.clone();

                                let result = task::spawn_blocking(move || {
                                    worker.install(move || {
                                        block_template.set_difficulty_target(share_difficulty);
                                        Block::mine(block_template, &terminator, &mut thread_rng())
                                    })
                                })
                                .await;

                                status.update(State::Ready);

                                match result {
                                    Ok(Ok(block)) => {
                                        debug!(
                                            "Miner has found block which meets share target {} ({})",
                                            block.height(),
                                            block.hash()
                                        );

                                        // Propose it to the pool.
                                        if let Err(error) = peers_router
                                            .send(PeersRequest::MessageSend(
                                                peer_ip,
                                                Message::SendShare(recipient, Data::Object(block)),
                                            ))
                                            .await
                                        {
                                            warn!("Could not send share to pool {}", error);
                                        }
                                    }
                                    Ok(Err(error)) => trace!("{}", error),
                                    Err(error) => trace!("{}", anyhow!("Could not mine next block {}", error)),
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
