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

use crate::{LedgerRequest, PeersRequest, State};
use snarkos_environment::{
    helpers::{NodeType, Status},
    network::{Data, Message},
    Environment,
};
use snarkos_storage::{storage::Storage, ProverState};
use snarkvm::dpc::{posw::PoSWProof, prelude::*};

use anyhow::{anyhow, Result};
use rand::thread_rng;
use std::{
    net::SocketAddr,
    path::Path,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    task,
};

/// Shorthand for the parent half of the `Prover` message channel.
pub type ProverRouter<N> = mpsc::Sender<ProverRequest<N>>;
/// Shorthand for the child half of the `Prover` message channel.
pub type ProverHandler<N> = mpsc::Receiver<ProverRequest<N>>;

/// The miner heartbeat in seconds.
const MINER_HEARTBEAT_IN_SECONDS: Duration = Duration::from_secs(2);

///
/// An enum of requests that the `Prover` struct processes.
///
#[derive(Debug)]
pub enum ProverRequest<N: Network> {
    /// PoolRequest := (peer_ip, share_difficulty, block_template)
    PoolRequest(SocketAddr, u64, BlockTemplate<N>),
    /// MemoryPoolClear := (block)
    MemoryPoolClear(Option<Block<N>>),
    /// UnconfirmedTransaction := (peer_ip, transaction)
    UnconfirmedTransaction(SocketAddr, Transaction<N>),
}

///
/// A prover for a specific network on the node server.
///
pub struct Prover<N: Network, E: Environment> {
    /// The state storage of the prover.
    prover_state: Arc<ProverState<N>>,
    /// The IP address of the connected pool.
    pool: Option<SocketAddr>,
    /// The prover router of the node.
    prover_router: ProverRouter<N>,
    /// The pool of unconfirmed transactions.
    memory_pool: Arc<RwLock<MemoryPool<N>>>,
    /// The shared state of the owning node.
    state: Arc<State<N, E>>,
}

impl<N: Network, E: Environment> Prover<N, E> {
    /// Initializes a new instance of the prover, paired with its handler.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        path: P,
        pool_ip: Option<SocketAddr>,
        state: Arc<State<N, E>>,
    ) -> Result<(Self, mpsc::Receiver<ProverRequest<N>>)> {
        // Initialize an mpsc channel for sending requests to the `Prover` struct.
        let (prover_router, prover_handler) = mpsc::channel(1024);
        // Initialize the prover.
        let prover = Self {
            prover_state: Arc::new(ProverState::open::<S, P>(path, false)?),
            pool: pool_ip,
            prover_router,
            memory_pool: Arc::new(RwLock::new(MemoryPool::new())),
            state,
        };

        Ok((prover, prover_handler))
    }

    pub async fn initialize_miner(&self) {
        // Initialize the miner, if the node type is a miner.
        if E::NODE_TYPE == NodeType::Miner && self.pool.is_none() {
            self.state.prover().start_miner().await;
        }
    }

    pub async fn initialize_pooling(&self) {
        // Initialize the prover, if the node type is a prover.
        if E::NODE_TYPE == NodeType::Prover && self.pool.is_some() {
            let state = self.state.clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None, // No need to provide an id, as the task will run indefinitely.
                task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    loop {
                        // Sleep for `1` second.
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                        // TODO (howardwu): Check that the prover is connected to the pool before proceeding.
                        //  Currently we use a sleep function to probabilistically ensure the peer is connected.
                        if !E::terminator().load(Ordering::SeqCst) && !E::status().is_peering() && !E::status().is_mining() {
                            state.prover().send_pool_register().await;
                        }
                    }
                }),
            );

            // Wait until the operator handler is ready.
            let _ = handler.await;
        }
    }

    pub async fn initialize_pool_connection_loop(&self, pool_ip: Option<SocketAddr>) {
        // TODO (howardwu): This is a hack for the prover.
        // Check that the prover is connected to the pool before sending a PoolRegister message.
        if let Some(pool_ip) = pool_ip {
            let peers_router = self.state.peers().router().clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None, // No need to provide an id, as the task will run indefinitely.
                task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    loop {
                        // Initialize the connection process.
                        let (router, handler) = oneshot::channel();
                        // Route a `Connect` request to the pool.
                        if let Err(error) = peers_router.send(PeersRequest::Connect(pool_ip, router)).await {
                            trace!("[Connect] {}", error);
                        }
                        // Wait until the connection task is initialized.
                        let _ = handler.await;

                        // Sleep for `30` seconds.
                        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    }
                }),
            );

            // Wait until the prover handler is ready.
            let _ = handler.await;
        }
    }

    /// Returns an instance of the prover router.
    pub fn router(&self) -> &ProverRouter<N> {
        &self.prover_router
    }

    /// Returns an instance of the memory pool.
    pub fn memory_pool(&self) -> Arc<RwLock<MemoryPool<N>>> {
        self.memory_pool.clone()
    }

    /// Returns all coinbase records in storage.
    pub fn to_coinbase_records(&self) -> Vec<(u32, Record<N>)> {
        self.prover_state.to_coinbase_records()
    }

    ///
    /// Performs the given `request` to the prover.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: ProverRequest<N>) {
        match request {
            ProverRequest::PoolRequest(operator_ip, share_difficulty, block_template) => {
                // Process the pool request message.
                self.process_pool_request(operator_ip, share_difficulty, block_template).await;
            }
            ProverRequest::MemoryPoolClear(block) => match block {
                Some(block) => self.memory_pool.write().await.remove_transactions(block.transactions()),
                None => *self.memory_pool.write().await = MemoryPool::new(),
            },
            ProverRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Ensure the node is not peering.
                if !E::status().is_peering() {
                    // Process the unconfirmed transaction.
                    self.add_unconfirmed_transaction(peer_ip, transaction).await
                }
            }
        }
    }

    ///
    /// Sends a `PoolRegister` message to the pool IP address.
    ///
    async fn send_pool_register(&self) {
        if E::NODE_TYPE == NodeType::Prover {
            if let Some(recipient) = self.state.address {
                if let Some(pool_ip) = self.pool {
                    // Proceed to register the prover to receive a block template.
                    let request = PeersRequest::MessageSend(pool_ip, Message::PoolRegister(recipient));
                    if let Err(error) = self.state.peers().router().send(request).await {
                        warn!("[PoolRegister] {}", error);
                    }
                } else {
                    error!("Missing pool IP address. Please specify a pool IP address in order to run the prover");
                }
            } else {
                error!("Missing prover address. Please specify an Aleo address in order to prove");
            }
        }
    }

    ///
    /// Processes a `PoolRequest` message from a pool operator.
    ///
    async fn process_pool_request(&self, operator_ip: SocketAddr, share_difficulty: u64, block_template: BlockTemplate<N>) {
        if E::NODE_TYPE == NodeType::Prover {
            if let Some(recipient) = self.state.address {
                if let Some(pool_ip) = self.pool {
                    // Refuse work from any pool other than the registered one.
                    if pool_ip == operator_ip {
                        // If `terminator` is `false` and the status is not `Peering` or `Mining`
                        // already, mine the next block.
                        if !E::terminator().load(Ordering::SeqCst) && !E::status().is_peering() && !E::status().is_mining() {
                            // Set the status to `Mining`.
                            E::status().update(Status::Mining);

                            let block_height = block_template.block_height();
                            let block_template = block_template.clone();

                            let result = task::spawn_blocking(move || {
                                E::thread_pool().install(move || {
                                    loop {
                                        let block_header =
                                            BlockHeader::mine_once_unchecked(&block_template, E::terminator(), &mut thread_rng())?;

                                        // Ensure the share difficulty target is met.
                                        if N::posw().verify(
                                            block_header.height(),
                                            share_difficulty,
                                            &[*block_header.to_header_root().unwrap(), *block_header.nonce()],
                                            block_header.proof(),
                                        ) {
                                            return Ok::<(N::PoSWNonce, PoSWProof<N>, u64), anyhow::Error>((
                                                block_header.nonce(),
                                                block_header.proof().clone(),
                                                block_header.proof().to_proof_difficulty()?,
                                            ));
                                        }
                                    }
                                })
                            })
                            .await;

                            E::status().update(Status::Ready);

                            match result {
                                Ok(Ok((nonce, proof, proof_difficulty))) => {
                                    info!(
                                        "Prover successfully mined a share for unconfirmed block {} with proof difficulty of {}",
                                        block_height, proof_difficulty
                                    );

                                    // Send a `PoolResponse` to the operator.
                                    let message = Message::PoolResponse(recipient, nonce, Data::Object(proof));
                                    if let Err(error) = self
                                        .state
                                        .peers()
                                        .router()
                                        .send(PeersRequest::MessageSend(operator_ip, message))
                                        .await
                                    {
                                        warn!("[PoolResponse] {}", error);
                                    }
                                }
                                Ok(Err(error)) => trace!("{}", error),
                                Err(error) => trace!("{}", anyhow!("Failed to mine the next block {}", error)),
                            }
                        }
                    }
                } else {
                    error!("Missing pool IP address. Please specify a pool IP address in order to run the prover");
                }
            } else {
                error!("Missing prover address. Please specify an Aleo address in order to prove");
            }
        }
    }

    ///
    /// Adds the given unconfirmed transaction to the memory pool.
    ///
    async fn add_unconfirmed_transaction(&self, peer_ip: SocketAddr, transaction: Transaction<N>) {
        // Process the unconfirmed transaction.
        trace!("Received unconfirmed transaction {} from {}", transaction.transaction_id(), peer_ip);
        // Ensure the unconfirmed transaction is new.
        if let Ok(false) = self.state.ledger().reader().contains_transaction(&transaction.transaction_id()) {
            debug!("Adding unconfirmed transaction {} to memory pool", transaction.transaction_id());
            // Attempt to add the unconfirmed transaction to the memory pool.
            match self.memory_pool.write().await.add_transaction(&transaction) {
                Ok(()) => {
                    // Upon success, propagate the unconfirmed transaction to the connected peers.
                    let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedTransaction(Data::Object(transaction)));
                    if let Err(error) = self.state.peers().router().send(request).await {
                        warn!("[UnconfirmedTransaction] {}", error);
                    }
                }
                Err(error) => error!("{}", error),
            }
        }
    }

    ///
    /// Initialize the miner, if the node type is a miner.
    ///
    async fn start_miner(&self) {
        // Initialize a new instance of the miner.
        if E::NODE_TYPE == NodeType::Miner && self.pool.is_none() {
            if let Some(recipient) = self.state.address {
                // Initialize the prover process.
                let (router, handler) = oneshot::channel();
                let state = self.state.clone();
                let prover_state = self.prover_state.clone();
                let local_ip = state.local_ip;
                E::resources().register_task(
                    None, // No need to provide an id, as the task will run indefinitely.
                    task::spawn(async move {
                        // Notify the outer function that the task is ready.
                        let _ = router.send(());
                        loop {
                            // If `terminator` is `false` and the status is not `Peering` or `Mining` already, mine the next block.
                            if !E::terminator().load(Ordering::SeqCst) && !E::status().is_peering() && !E::status().is_mining() {
                                // Set the status to `Mining`.
                                E::status().update(Status::Mining);

                                // Prepare the unconfirmed transactions and dependent objects.
                                let prover_state = prover_state.clone();
                                let canon = state.ledger().reader().clone(); // This is *safe* as the ledger only reads.
                                let unconfirmed_transactions = state.prover().memory_pool.read().await.transactions();
                                let ledger_router = state.ledger().router().clone();

                                // Procure a resource id to register the task with, as it might be terminated at any point in time.
                                let mining_task_id = E::resources().procure_id();
                                E::resources().register_task(
                                    Some(mining_task_id),
                                    task::spawn(async move {
                                        // Mine the next block.
                                        let result = task::spawn_blocking(move || {
                                            E::thread_pool().install(move || {
                                                canon.mine_next_block(
                                                    recipient,
                                                    E::COINBASE_IS_PUBLIC,
                                                    &unconfirmed_transactions,
                                                    E::terminator(),
                                                    &mut thread_rng(),
                                                )
                                            })
                                        })
                                        .await
                                        .map_err(|e| e.into());

                                        // Set the status to `Ready`.
                                        E::status().update(Status::Ready);

                                        match result {
                                            Ok(Ok((block, coinbase_record))) => {
                                                debug!("Miner has found unconfirmed block {} ({})", block.height(), block.hash());
                                                // Store the coinbase record.
                                                if let Err(error) = prover_state.add_coinbase_record(block.height(), coinbase_record) {
                                                    warn!("[Miner] Failed to store coinbase record - {}", error);
                                                }

                                                // Broadcast the next block.
                                                let request = LedgerRequest::UnconfirmedBlock(local_ip, block);
                                                if let Err(error) = ledger_router.send(request).await {
                                                    warn!("Failed to broadcast mined block - {}", error);
                                                }
                                            }
                                            Ok(Err(error)) | Err(error) => trace!("{}", error),
                                        }

                                        E::resources().deregister(mining_task_id);
                                    }),
                                );
                            }
                            // Proceed to sleep for a preset amount of time.
                            tokio::time::sleep(MINER_HEARTBEAT_IN_SECONDS).await;
                        }
                    }),
                );

                // Wait until the miner task is ready.
                let _ = handler.await;
            } else {
                error!("Missing miner address. Please specify an Aleo address in order to mine");
            }
        }
    }
}
