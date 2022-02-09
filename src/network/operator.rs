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

use crate::{
    helpers::NodeType,
    Data,
    Environment,
    LedgerReader,
    LedgerRequest,
    LedgerRouter,
    Message,
    PeersRequest,
    PeersRouter,
    ProverRouter,
};
use snarkos_storage::{storage::Storage, OperatorState};
use snarkvm::dpc::{prelude::*, PoSWProof};

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
    /// PoolRegister := (peer_ip, prover_address)
    PoolRegister(SocketAddr, Address<N>),
    /// PoolResponse := (peer_ip, prover_address, nonce, proof)
    PoolResponse(SocketAddr, Address<N>, N::PoSWNonce, PoSWProof<N>),
}

/// The predefined base share difficulty.
const BASE_SHARE_DIFFICULTY: u64 = u64::MAX;
/// The operator heartbeat in seconds.
const HEARTBEAT_IN_SECONDS: Duration = Duration::from_secs(1);

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
    /// A list of provers and their associated state := (last_submitted, share_difficulty)
    provers: RwLock<HashMap<Address<N>, (Instant, u64)>>,
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
    #[allow(clippy::too_many_arguments)]
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
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
            E::tasks().append(task::spawn(async move {
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
                E::tasks().append(task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // TODO (julesdesmit): Add logic to the loop to retarget share difficulty.
                    loop {
                        // Determine if the current block template is stale.
                        let is_block_template_stale = match &*operator.block_template.read().await {
                            Some(template) => operator.ledger_reader.latest_block_height().saturating_add(1) != template.block_height(),
                            None => true,
                        };

                        // Update the block template if it is stale.
                        if is_block_template_stale {
                            // Construct a new block template.
                            let transactions = operator.memory_pool.read().await.transactions();
                            let ledger_reader = operator.ledger_reader.clone();
                            let result = task::spawn_blocking(move || {
                                E::thread_pool().install(move || {
                                    match ledger_reader.get_block_template(
                                        recipient,
                                        E::COINBASE_IS_PUBLIC,
                                        &transactions,
                                        &mut thread_rng(),
                                    ) {
                                        Ok(block_template) => Ok(block_template),
                                        Err(error) => Err(format!("Failed to produce a new block template: {}", error)),
                                    }
                                })
                            })
                            .await;

                            // Update the block template.
                            match result {
                                Ok(Ok(block_template)) => {
                                    // Acquire the write lock to update the block template.
                                    *operator.block_template.write().await = Some(block_template);
                                    // Clear the set of known nonces.
                                    operator.known_nonces.write().await.clear();
                                }
                                Ok(Err(error_message)) => error!("{}", error_message),
                                Err(error) => error!("{}", error),
                            };
                        }

                        // Proceed to sleep for a preset amount of time.
                        tokio::time::sleep(HEARTBEAT_IN_SECONDS).await;
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
    pub fn to_shares(&self) -> Vec<((u32, Record<N>), HashMap<Address<N>, u64>)> {
        self.state.to_shares()
    }

    /// Returns the shares for a specific block, given the block height and coinbase record commitment.
    pub fn get_shares_for_block(&self, block_height: u32, coinbase_record: Record<N>) -> Result<HashMap<Address<N>, u64>> {
        self.state.get_shares_for_block(block_height, coinbase_record)
    }

    /// Returns the shares for a specific prover, given a ledger and the prover address.
    pub fn get_shares_for_prover(&self, prover: &Address<N>) -> u64 {
        self.state.get_shares_for_prover(&self.ledger_reader, prover)
    }

    ///
    /// Returns a list of all provers which have submitted shares to this operator.
    ///
    pub fn get_provers(&self) -> Vec<Address<N>> {
        self.state.get_provers()
    }

    ///
    /// Performs the given `request` to the operator.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: OperatorRequest<N>) {
        match request {
            OperatorRequest::PoolRegister(peer_ip, address) => {
                if let Some(block_template) = self.block_template.read().await.clone() {
                    // Ensure that we're connected to the prover.
                    // TODO(julesdesmit): this is a hack to ensure connections, and we should do
                    // this more efficiently, ideally speaking.
                    let (router, handler) = oneshot::channel();
                    if let Err(error) = self
                        .peers_router
                        .send(PeersRequest::Connect(
                            peer_ip,
                            self.ledger_reader.clone(),
                            self.ledger_router.clone(),
                            self.operator_router.clone(),
                            self.prover_router.clone(),
                            router,
                        ))
                        .await
                    {
                        trace!("[Connect] {}", error);
                    }
                    let _ = handler.await;

                    // Ensure this prover exists in the list first, and retrieve their share difficulty.
                    let share_difficulty = self
                        .provers
                        .write()
                        .await
                        .entry(address)
                        .or_insert((Instant::now(), BASE_SHARE_DIFFICULTY))
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
            OperatorRequest::PoolResponse(peer_ip, prover, nonce, proof) => {
                if let Some(block_template) = self.block_template.read().await.clone() {
                    // Ensure the given nonce from the prover is new.
                    if self.known_nonces.read().await.contains(&nonce) {
                        warn!("[PoolResponse] Peer {} sent a duplicate share", peer_ip);
                        // TODO (julesdesmit): punish?
                        return;
                    }

                    // Update known nonces.
                    self.known_nonces.write().await.insert(nonce);

                    // Retrieve the share difficulty for the given prover.
                    let share_difficulty = {
                        let provers = self.provers.read().await.clone();
                        match provers.get(&prover) {
                            Some((_, share_difficulty)) => *share_difficulty,
                            None => {
                                self.provers.write().await.insert(prover, (Instant::now(), BASE_SHARE_DIFFICULTY));
                                BASE_SHARE_DIFFICULTY
                            }
                        }
                    };

                    // Ensure the share difficulty target is met, and the PoSW proof is valid.
                    let block_height = block_template.block_height();
                    if !N::posw().verify(
                        block_height,
                        share_difficulty,
                        &[*block_template.to_header_root().unwrap(), *nonce],
                        &proof,
                    ) {
                        warn!("[PoolResponse] PoSW proof verification failed");
                        return;
                    }

                    // Update the internal state for this prover.
                    if let Some(ref mut prover) = self.provers.write().await.get_mut(&prover) {
                        prover.0 = Instant::now();
                    } else {
                        error!("Prover should have existing info");
                        return;
                    }

                    // Increment the share count for the prover.
                    let coinbase_record = block_template.coinbase_record().clone();
                    match self.state.increment_share(block_height, coinbase_record, &prover) {
                        Ok(..) => info!(
                            "Operator has received a valid share from {} ({}) for block {}",
                            prover, peer_ip, block_height,
                        ),
                        Err(error) => error!("{}", error),
                    }

                    // If the block has satisfactory difficulty and is valid, proceed to broadcast it.
                    let previous_block_hash = block_template.previous_block_hash();
                    let transactions = block_template.transactions().clone();
                    if let Ok(block_header) = BlockHeader::<N>::from(
                        block_template.previous_ledger_root(),
                        block_template.transactions().transactions_root(),
                        BlockHeaderMetadata::new(&block_template),
                        nonce,
                        proof,
                    ) {
                        if let Ok(block) = Block::from(previous_block_hash, block_header, transactions) {
                            info!("Operator has found unconfirmed block {} ({})", block.height(), block.hash());
                            let request = LedgerRequest::UnconfirmedBlock(self.local_ip, block, self.prover_router.clone());
                            if let Err(error) = self.ledger_router.send(request).await {
                                warn!("Failed to broadcast mined block - {}", error);
                            }
                        }
                    }
                } else {
                    warn!("[PoolResponse] No current block template exists");
                }
            }
        }
    }
}
