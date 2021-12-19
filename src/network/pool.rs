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
    helpers::Tasks,
    Data,
    Environment,
    LedgerReader,
    LedgerRequest,
    LedgerRouter,
    Message,
    NodeType,
    PeersRequest,
    PeersRouter,
    ProverRouter,
};
use snarkos_storage::{storage::Storage, PoolState};
use snarkvm::{algorithms::crh::sha256d_to_u64, dpc::prelude::*, utilities::ToBytes};

use anyhow::Result;
use rand::thread_rng;
use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc, time::Duration};
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    task,
    task::JoinHandle,
};

/// Shorthand for the parent half of the `Pool` message channel.
pub(crate) type PoolRouter<N> = mpsc::Sender<PoolRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Pool` message channel.
type PoolHandler<N> = mpsc::Receiver<PoolRequest<N>>;

///
/// An enum of requests that the `Pool` struct processes.
///
#[derive(Debug)]
pub enum PoolRequest<N: Network> {
    /// ProposedBlock := (peer_ip, proposed_block, worker_address)
    ProposedBlock(SocketAddr, Block<N>, Address<N>),
    /// GetCurrentBlockTemplate := (peer_ip, worker_address)
    GetCurrentBlockTemplate(SocketAddr, Address<N>),
    /// BlockHeightClear := (block_height)
    BlockHeightClear(u32),
}

///
/// A pool for a specific network on the node server.
///
#[derive(Debug)]
pub struct Pool<N: Network, E: Environment> {
    /// The address of the pool.
    pool_address: Option<Address<N>>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The state storage of the pool.
    state: Arc<PoolState<N>>,
    /// The pool router of the node.
    pool_router: PoolRouter<N>,
    /// The pool of unconfirmed transactions.
    memory_pool: Arc<RwLock<MemoryPool<N>>>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The ledger state of the node.
    ledger_reader: LedgerReader<N>,
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N>,
    /// The prover router of the node.
    prover_router: ProverRouter<N>,
    /// The current block template that is being mined on by the pool.
    current_template: RwLock<Option<BlockTemplate<N>>>,
    /// Peripheral information on each known worker.
    /// WorkerInfo := (last_submitted, share_difficulty, shares_submitted_since_reset)
    worker_info: RwLock<HashMap<Address<N>, (i64, u64, u32)>>,
}

impl<N: Network, E: Environment> Pool<N, E> {
    /// Initializes a new instance of the pool.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        tasks: &Tasks<JoinHandle<()>>,
        path: P,
        pool_address: Option<Address<N>>,
        local_ip: SocketAddr,
        memory_pool: Arc<RwLock<MemoryPool<N>>>,
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
    ) -> Result<Arc<Self>> {
        // Initialize an mpsc channel for sending requests to the `Pool` struct.
        let (pool_router, mut pool_handler) = mpsc::channel(1024);

        // Initialize the pool.
        let pool = Arc::new(Self {
            pool_address,
            local_ip,
            state: Arc::new(PoolState::open_writer::<S, P>(path)?),
            pool_router,
            memory_pool,
            peers_router,
            ledger_reader,
            ledger_router,
            prover_router,
            current_template: RwLock::new(None),
            worker_info: RwLock::new(HashMap::new()),
        });

        if E::NODE_TYPE == NodeType::Operator {
            // Initialize the handler for the pool.
            let pool_clone = pool.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // TODO (julesdesmit): add loop which retargets share difficulty.
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a pool request.
                while let Some(request) = pool_handler.recv().await {
                    pool_clone.update(request).await;
                }
            }));
            // Wait until the pool handler is ready.
            let _ = handler.await;

            // Set up an update loop for the block template.
            let pool_clone = pool.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a pool request.
                let recipient = pool_clone
                    .pool_address
                    .expect("A pool should have an available Aleo address at all times");
                loop {
                    let mut current_template = pool_clone.current_template.write().await;
                    match &*current_template {
                        Some(t) => {
                            if pool_clone.ledger_reader.latest_block_height() != t.block_height() - 1 {
                                *current_template = Some(
                                    pool_clone
                                        .generate_block_template(recipient)
                                        .await
                                        .expect("Should be able to generate a block template"),
                                );
                            }
                        }
                        None => {
                            *current_template = Some(
                                pool_clone
                                    .generate_block_template(recipient)
                                    .await
                                    .expect("Should be able to generate a block template"),
                            );
                        }
                    };
                    drop(current_template); // Release lock, to avoid recursively locking.

                    // Sleep for `5` seconds.
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }));
            // Wait until the pool handler is ready.
            let _ = handler.await;
        }

        Ok(pool)
    }

    /// Returns an instance of the pool router.
    pub fn router(&self) -> PoolRouter<N> {
        self.pool_router.clone()
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u64>)> {
        self.state.to_shares()
    }

    ///
    /// Performs the given `request` to the pool.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: PoolRequest<N>) {
        match request {
            PoolRequest::ProposedBlock(peer_ip, mut block, worker_address) => {
                if let Some(current_template) = &*self.current_template.read().await {
                    // Check that the block is relevant.
                    if self.ledger_reader.latest_block_height().saturating_add(1) != block.height() {
                        warn!("[ProposedBlock] Peer {} sent a stale candidate block.", peer_ip);
                        return;
                    }

                    // Check that the block's coinbase transaction owner is the pool address.
                    let records = match block.to_coinbase_transaction() {
                        Ok(tx) => {
                            let coinbase_records: Vec<Record<N>> = tx.to_records().collect();
                            let valid_owner = coinbase_records.iter().any(|r| Some(r.owner()) == self.pool_address);

                            if !valid_owner {
                                warn!("[ProposedBlock] Peer {} sent a candidate block with an invalid owner.", peer_ip);
                                return;
                            }

                            coinbase_records
                        }
                        Err(err) => {
                            warn!("[ProposedBlock] {}", err);
                            return;
                        }
                    };

                    // Determine the score to add for the miner.
                    let proof_bytes = match block.header().proof() {
                        Some(proof) => match proof.to_bytes_le() {
                            Ok(bytes) => bytes,
                            Err(err) => {
                                warn!("[ProposedBlock] {}", err);
                                return;
                            }
                        },
                        None => {
                            warn!("[ProposedBlock] Peer {} sent a candidate block with a missing proof.", peer_ip);
                            return;
                        }
                    };

                    let hash_difficulty = sha256d_to_u64(&proof_bytes);
                    let share_difficulty = {
                        let mut info = self.worker_info.write().await;
                        match info.get(&worker_address) {
                            Some((_, share_difficulty, _)) => *share_difficulty,
                            None => {
                                let share_difficulty = current_template.difficulty_target().saturating_mul(50);
                                info.insert(worker_address, (chrono::Utc::now().timestamp(), share_difficulty, 0));

                                share_difficulty
                            }
                        }
                    };

                    if hash_difficulty > share_difficulty {
                        warn!("[ProposedBlock] faulty share submitted by {}", worker_address);
                        return;
                    }

                    // Update the score for the worker.
                    // TODO: add round stuff
                    // TODO: ensure shares can not be resubmitted
                    if let Err(error) = self.state.add_shares(block.height(), &worker_address, 1) {
                        warn!("[ProposedBlock] {}", error);
                    }

                    debug!(
                        "Pool has received valid share {} ({}) - {} / {}",
                        block.height(),
                        block.hash(),
                        worker_address,
                        peer_ip
                    );

                    {
                        // Update info for this worker.
                        let mut info = self.worker_info.write().await;
                        let mut worker_info = *info.get_mut(&worker_address).expect("worker should have existing info");
                        worker_info.0 = chrono::Utc::now().timestamp();
                        worker_info.2 += 1;
                        info.insert(worker_address, worker_info);
                    }

                    // Since a worker will swap out the difficulty target for their share target,
                    // let's put it back to the original value before checking the POSW for true
                    // validity.
                    let difficulty_target = current_template.difficulty_target();
                    block.set_difficulty_target(difficulty_target);

                    // If the block is valid, broadcast it.
                    if block.is_valid() {
                        debug!("Mining pool has found unconfirmed block {} ({})", block.height(), block.hash());

                        // Store coinbase record(s)
                        records.iter().for_each(|r| {
                            if let Err(error) = self.state.add_coinbase_record(block.height(), r.clone()) {
                                warn!("Could not store coinbase record {}", error);
                            }
                        });

                        // Broadcast the next block.
                        let request = LedgerRequest::UnconfirmedBlock(self.local_ip, block, self.prover_router.clone());
                        if let Err(error) = self.ledger_router.send(request).await {
                            warn!("Failed to broadcast mined block - {}", error);
                        }
                    }
                } else {
                    warn!("[ProposedBlock] No current template exists");
                }
            }
            PoolRequest::BlockHeightClear(block_height) => {
                // Remove the shares for the given block height.
                if let Err(error) = self.state.remove_shares(block_height) {
                    warn!("[BlockHeightClear] {}", error);
                }
            }
            PoolRequest::GetCurrentBlockTemplate(peer_ip, address) => {
                if let Some(current_template) = &*self.current_template.read().await {
                    // Ensure this worker exists in the info list first, so we can get their share
                    // difficulty.
                    let share_difficulty = self
                        .worker_info
                        .write()
                        .await
                        .entry(address)
                        .or_insert((
                            chrono::Utc::now().timestamp(),
                            current_template.difficulty_target().saturating_mul(50),
                            0,
                        ))
                        .1;

                    if let Err(error) = self
                        .peers_router
                        .send(PeersRequest::MessageSend(
                            peer_ip,
                            Message::BlockTemplate(share_difficulty, Data::Object(current_template.clone())),
                        ))
                        .await
                    {
                        warn!("[ProposedBlock] {}", error);
                    }
                } else {
                    warn!("[ProposedBlock] No current block template exists");
                }
            }
        }
    }

    async fn generate_block_template(&self, recipient: Address<N>) -> Result<BlockTemplate<N>> {
        let unconfirmed_transactions = self.memory_pool.read().await.transactions();
        let (block_template, _) =
            self.ledger_reader
                .get_block_template(recipient, E::COINBASE_IS_PUBLIC, &unconfirmed_transactions, &mut thread_rng())?;
        Ok(block_template)
    }
}
