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
use snarkvm::{
    algorithms::{crh::sha256d_to_u64, SNARK},
    dpc::prelude::*,
    utilities::{FromBytes, ToBytes},
};

use anyhow::Result;
use rand::thread_rng;
use std::{
    collections::{HashMap, HashSet},
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

const UPDATE_DELAY: Duration = Duration::from_secs(5);

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
    /// PoolResponse := (peer_ip, proposed_block_header, worker_address)
    PoolResponse(SocketAddr, BlockHeader<N>, Address<N>),
}

/// The predefined base share difficulty.
const BASE_SHARE_DIFFICULTY: u64 = u64::MAX / 2u64.pow(1);

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
    /// The current block template that is being mined on by the operator.
    block_template: RwLock<Option<BlockTemplate<N>>>,
    /// A list of provers and their associated state := (last_submitted, share_difficulty, shares_submitted_since_reset)
    provers: RwLock<HashMap<Address<N>, (Instant, u64, u32)>>,
    /// A list of the known nonces for the current round.
    known_nonces: RwLock<HashSet<N::PoSWNonce>>,
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
            block_template: RwLock::new(None),
            provers: Default::default(),
            known_nonces: Default::default(),
            operator_router,
            memory_pool,
            peers_router,
            ledger_reader,
            ledger_router,
            prover_router,
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
        }

        if E::NODE_TYPE == NodeType::Operator {
            if let Some(recipient) = operator.address {
                // Initialize an update loop for the block template.
                let operator = operator.clone();
                let (router, handler) = oneshot::channel();
                tasks.append(task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // TODO (julesdesmit): Add logic to the loop to retarget share difficulty.
                    loop {
                        // Determine if the current block template is stale.
                        let is_template_stale = match &*operator.block_template.read().await {
                            Some(template) => operator.ledger_reader.latest_block_height().saturating_add(1) != template.block_height(),
                            None => true,
                        };

                        // Update the block template if it is stale.
                        if is_template_stale {
                            // Construct a new block template.
                            let transactions = operator.memory_pool.read().await.transactions();
                            let (block_template, _) = operator
                                .ledger_reader
                                .get_block_template(recipient, E::COINBASE_IS_PUBLIC, &transactions, &mut thread_rng())
                                .expect("Should be able to generate a block template");

                            // Acquire the write lock to update the block template.
                            *operator.block_template.write().await = Some(block_template);

                            // Clear the known_nonces hash set.
                            operator.known_nonces.write().await.clear();
                        }

                        // Sleep for `5` seconds.
                        tokio::time::sleep(UPDATE_DELAY).await;
                    }
                }));
                // Wait until the operator handler is ready.
                let _ = handler.await;
            } else {
                error!("Missing operator address. Please specify an Aleo address in order to operate a pool");
            }
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
                if let Some(block_template) = self.block_template.read().await.clone() {
                    // Ensure this prover exists in the list first, and retrieve their share difficulty.
                    let share_difficulty = self
                        .provers
                        .write()
                        .await
                        .entry(address)
                        .or_insert((Instant::now(), BASE_SHARE_DIFFICULTY, 0))
                        .1;

                    // Route a `PoolRequest` to the peer.
                    let message = Message::PoolRequest(share_difficulty, Data::Object(block_template));
                    if let Err(error) = self.peers_router.send(PeersRequest::MessageSend(peer_ip, message)).await {
                        warn!("[PoolRequest] {}", error);
                    }
                } else {
                    warn!("[PoolRegister] No current block template exists");
                }
            }
            OperatorRequest::PoolResponse(peer_ip, block_header, prover_address) => {
                if let Some(block_template) = self.block_template.read().await.clone() {
                    // Check that the block is relevant.
                    if self.ledger_reader.latest_block_height().saturating_add(1) != block_header.height() {
                        warn!("[PoolResponse] Peer {} sent a stale candidate block.", peer_ip);
                        return;
                    }

                    // Ensure the nonce hasn't been seen before.
                    if self.known_nonces.read().await.contains(&block_header.nonce()) {
                        warn!("[PoolResponse] Peer {} sent a duplicate share", peer_ip);
                        // TODO (julesdesmit): punish?
                        return;
                    }

                    // Reconstruct the block.
                    let previous_block_hash = block_template.previous_block_hash();
                    let transactions = block_template.transactions().clone();

                    if let Ok(block) = Block::from_unchecked(previous_block_hash, block_header.clone(), transactions) {
                        // Ensure the proof is valid.
                        let proof = match block.header().proof() {
                            Some(proof) => proof,
                            None => {
                                warn!("[PoolResponse] proof is missing on header");
                                return;
                            }
                        };

                        // NOTE (julesdesmit): Unwraps are here for brevity's sake, and since we do
                        // it exactly the same in snarkVM, I don't see why we shouldn't use them here.
                        let inputs = vec![
                            N::InnerScalarField::read_le(&block_header.to_header_root().unwrap().to_bytes_le().unwrap()[..]).unwrap(),
                            *block_header.nonce(),
                        ];

                        if !<<N as Network>::PoSWSNARK as SNARK>::verify(N::posw().verifying_key(), &inputs, proof).unwrap() {
                            warn!("[PoolResponse] PoSW proof verification failed");
                            return;
                        }

                        // Retrieve the coinbase transaction records.
                        let coinbase_records = match block.to_coinbase_transaction() {
                            Ok(transaction) => {
                                // Ensure the owner of the coinbase transaction in the block is the operator address.
                                let coinbase_records: Vec<Record<N>> =
                                    transaction.to_records().filter(|r| Some(r.owner()) == self.address).collect();
                                if coinbase_records.is_empty() {
                                    warn!("[PoolResponse] Peer {} sent a candidate block with an incorrect owner.", peer_ip);
                                    return;
                                }
                                coinbase_records
                            }
                            Err(error) => {
                                warn!("[PoolResponse] {}", error);
                                return;
                            }
                        };

                        // Ensure the block contains a difficulty that is at least the share difficulty.
                        let proof_bytes = match proof.to_bytes_le() {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                warn!("[PoolResponse] {}", error);
                                return;
                            }
                        };

                        let proof_difficulty = sha256d_to_u64(&proof_bytes);
                        let share_difficulty = {
                            let provers = self.provers.read().await.clone();
                            match provers.get(&prover_address) {
                                Some((_, share_difficulty, _)) => *share_difficulty,
                                None => {
                                    self.provers
                                        .write()
                                        .await
                                        .insert(prover_address, (Instant::now(), BASE_SHARE_DIFFICULTY, 0));
                                    BASE_SHARE_DIFFICULTY
                                }
                            }
                        };

                        if proof_difficulty > share_difficulty {
                            warn!("Block with insufficient share difficulty from {} ({})", peer_ip, prover_address);
                            return;
                        }

                        // Update the score for the prover.
                        // TODO: add round stuff
                        if let Err(error) = self.state.add_shares(block.height(), &prover_address, 1) {
                            error!("{}", error);
                        }

                        // Update known nonces.
                        self.known_nonces.write().await.insert(block.header().nonce());

                        // Update the internal state for this prover.
                        if let Some(ref mut prover) = self.provers.write().await.get_mut(&prover_address) {
                            prover.0 = Instant::now();
                            prover.2 += 1;
                        } else {
                            panic!("prover should have existing info");
                        }

                        info!(
                            "Operator received a valid share from {} ({}) for block {} ({})",
                            peer_ip,
                            prover_address,
                            block.height(),
                            block.hash(),
                        );

                        // If the block has satisfactory difficulty and is valid, proceed to broadcast it.
                        if let Ok(block) = Block::new(&block_template, block_header) {
                            info!("Operator has found unconfirmed block {} ({})", block.height(), block.hash());

                            // Store the coinbase record(s).
                            coinbase_records.iter().for_each(|r| {
                                if let Err(error) = self.state.add_coinbase_record(block.height(), r.clone()) {
                                    warn!("Could not store coinbase record - {}", error);
                                }
                            });

                            // Broadcast the unconfirmed block.
                            let request = LedgerRequest::UnconfirmedBlock(self.local_ip, block, self.prover_router.clone());
                            if let Err(error) = self.ledger_router.send(request).await {
                                warn!("Failed to broadcast mined block - {}", error);
                            }
                        }
                    } else {
                        warn!("[PoolResponse] Invalid block provided");
                    }
                } else {
                    warn!("[PoolResponse] No current block template exists");
                }
            }
        }
    }
}
