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
use snarkos_storage::{storage::Storage, OperatorState};
use snarkvm::{algorithms::crh::sha256d_to_u64, dpc::prelude::*, utilities::ToBytes};

use anyhow::Result;
use rand::thread_rng;
use std::{
    collections::HashMap,
    net::SocketAddr,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    task,
    task::JoinHandle,
};

/// Shorthand for the parent half of the `Operator` message channel.
pub(crate) type OperatorRouter<N> = mpsc::Sender<OperatorRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Operator` message channel.
type OperatorHandler<N> = mpsc::Receiver<OperatorRequest<N>>;

///
/// An enum of requests that the `Operator` struct processes.
///
#[derive(Debug)]
pub enum OperatorRequest<N: Network> {
    /// PoolRegister := (peer_ip, worker_address)
    PoolRegister(SocketAddr, Address<N>),
    /// ProposedBlock := (peer_ip, proposed_block, worker_address)
    ProposedBlock(SocketAddr, Block<N>, Address<N>),
}

/// The predefined base share difficulty.
const BASE_SHARE_DIFFICULTY: u64 = u64::MAX / 2u64.pow(8);

///
/// An operator for a program on a specific network in the node server.
///
#[derive(Debug)]
pub struct Operator<N: Network, E: Environment> {
    /// The address of the operator.
    address: Option<Address<N>>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The state storage of the operator.
    state: Arc<OperatorState<N>>,
    /// The operator router of the node.
    operator_router: OperatorRouter<N>,
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
    /// The current block template that is being mined on by the operator.
    block_template: RwLock<Option<BlockTemplate<N>>>,
    /// A list of provers and their associated state := (last_submitted, share_difficulty, shares_submitted_since_reset)
    provers: RwLock<HashMap<Address<N>, (Instant, u64, u32)>>,
}

impl<N: Network, E: Environment> Operator<N, E> {
    /// Initializes a new instance of the operator.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        tasks: &Tasks<JoinHandle<()>>,
        path: P,
        address: Option<Address<N>>,
        local_ip: SocketAddr,
        memory_pool: Arc<RwLock<MemoryPool<N>>>,
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
    ) -> Result<Arc<Self>> {
        // Initialize an mpsc channel for sending requests to the `Operator` struct.
        let (operator_router, mut operator_handler) = mpsc::channel(1024);

        // Initialize the operator.
        let operator = Arc::new(Self {
            address,
            local_ip,
            state: Arc::new(OperatorState::open_writer::<S, P>(path)?),
            operator_router,
            memory_pool,
            peers_router,
            ledger_reader,
            ledger_router,
            prover_router,
            block_template: RwLock::new(None),
            provers: RwLock::new(HashMap::new()),
        });

        if E::NODE_TYPE == NodeType::Operator {
            // Initialize the handler for the operator.
            let operator_clone = operator.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a operator request.
                while let Some(request) = operator_handler.recv().await {
                    operator_clone.update(request).await;
                }
            }));
            // Wait until the operator handler is ready.
            let _ = handler.await;

            // Set up an update loop for the block template.
            let operator_clone = operator.clone();
            let (router, handler) = oneshot::channel();
            tasks.append(task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a operator request.
                let recipient = operator_clone
                    .address
                    .expect("A pool should have an available Aleo address at all times");
                // TODO (julesdesmit): add loop which retargets share difficulty.
                loop {
                    let mut block_template = operator_clone.block_template.write().await;
                    match &*block_template {
                        Some(t) => {
                            if operator_clone.ledger_reader.latest_block_height() != t.block_height() - 1 {
                                *block_template = Some(
                                    operator_clone
                                        .generate_block_template(recipient)
                                        .await
                                        .expect("Should be able to generate a block template"),
                                );
                            }
                        }
                        None => {
                            *block_template = Some(
                                operator_clone
                                    .generate_block_template(recipient)
                                    .await
                                    .expect("Should be able to generate a block template"),
                            );
                        }
                    };
                    drop(block_template); // Release lock, to avoid recursively locking.

                    // Sleep for `5` seconds.
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }));
            // Wait until the pool handler is ready.
            let _ = handler.await;
        }

        Ok(operator)
    }

    /// Returns an instance of the operator router.
    pub fn router(&self) -> OperatorRouter<N> {
        self.operator_router.clone()
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u64>)> {
        self.state.to_shares()
    }

    ///
    /// Performs the given `request` to the operator.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: OperatorRequest<N>) {
        match request {
            OperatorRequest::PoolRegister(peer_ip, address) => {
                if let Some(block_template) = &*self.block_template.read().await {
                    // Ensure this prover exists in the list first, and retrieve their share difficulty.
                    let share_difficulty = self
                        .provers
                        .write()
                        .await
                        .entry(address)
                        .or_insert((Instant::now(), BASE_SHARE_DIFFICULTY, 0))
                        .1;

                    // Route a `PoolRequest` to the peer.
                    let message = Message::PoolRequest(share_difficulty, Data::Object(block_template.clone()));
                    if let Err(error) = self.peers_router.send(PeersRequest::MessageSend(peer_ip, message)).await {
                        warn!("[PoolRequest] {}", error);
                    }
                } else {
                    warn!("[GetBlockTemplate] No current block template exists");
                }
            }
            OperatorRequest::ProposedBlock(peer_ip, mut block, prover_address) => {
                if let Some(block_template) = &*self.block_template.read().await {
                    // Check that the block is relevant.
                    if self.ledger_reader.latest_block_height().saturating_add(1) != block.height() {
                        warn!("[ProposedBlock] Peer {} sent a stale candidate block.", peer_ip);
                        return;
                    }

                    // Check that the block's coinbase transaction owner is the pool address.
                    let records = match block.to_coinbase_transaction() {
                        Ok(tx) => {
                            let coinbase_records: Vec<Record<N>> = tx.to_records().collect();
                            let valid_owner = coinbase_records.iter().any(|r| Some(r.owner()) == self.address);

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
                        let mut provers = self.provers.write().await;
                        match provers.get(&prover_address) {
                            Some((_, share_difficulty, _)) => *share_difficulty,
                            None => {
                                provers.insert(prover_address, (Instant::now(), BASE_SHARE_DIFFICULTY, 0));
                                BASE_SHARE_DIFFICULTY
                            }
                        }
                    };

                    if hash_difficulty > share_difficulty {
                        warn!("[ProposedBlock] faulty share submitted by {}", prover_address);
                        return;
                    }

                    // Update the score for the worker.
                    // TODO: add round stuff
                    // TODO: ensure shares can not be resubmitted
                    if let Err(error) = self.state.add_shares(block.height(), &prover_address, 1) {
                        warn!("[ProposedBlock] {}", error);
                    }

                    debug!(
                        "Operator has received valid share {} ({}) - {} / {}",
                        block.height(),
                        block.hash(),
                        prover_address,
                        peer_ip
                    );

                    {
                        // Update the internal state for this prover.
                        let mut provers = self.provers.write().await;
                        let mut prover = *provers.get_mut(&prover_address).expect("worker should have existing info");
                        prover.0 = Instant::now();
                        prover.2 += 1;
                        provers.insert(prover_address, prover);
                    }

                    // Since a worker will swap out the difficulty target for their share target,
                    // let's put it back to the original value before checking the POSW for true
                    // validity.
                    let difficulty_target = block_template.difficulty_target();
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
