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

use crate::{state::NetworkState, LedgerReader};
use snarkos_environment::{
    helpers::{NodeType, State},
    network::{Data, Message},
    Environment,
};
use snarkos_storage::{storage::Storage, ProverState};
use snarkvm::dpc::{posw::PoSWProof, prelude::*};

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use rand::thread_rng;
use std::{
    net::SocketAddr,
    path::Path,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{
    sync::{oneshot, RwLock},
    task,
};

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
#[derive(Debug)]
pub struct Prover<N: Network, E: Environment> {
    network_state: OnceCell<NetworkState<N, E>>,
    /// The state storage of the prover.
    state: Arc<ProverState<N>>,
    /// The Aleo address of the prover.
    address: Option<Address<N>>,
    /// The IP address of the connected pool.
    pool: Option<SocketAddr>,
    /// The pool of unconfirmed transactions.
    memory_pool: Arc<RwLock<MemoryPool<N>>>,
    /// The ledger state of the node.
    ledger_reader: LedgerReader<N>,
}

impl<N: Network, E: Environment> Prover<N, E> {
    /// Initializes a new instance of the prover.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        path: P,
        address: Option<Address<N>>,
        local_ip: SocketAddr,
        pool_ip: Option<SocketAddr>,
        ledger_reader: LedgerReader<N>,
    ) -> Result<Arc<Self>> {
        // Initialize the prover.
        let prover = Arc::new(Self {
            network_state: OnceCell::new(),
            state: Arc::new(ProverState::open::<S, P>(path, false)?),
            address,
            pool: pool_ip,
            memory_pool: Arc::new(RwLock::new(MemoryPool::new())),
            ledger_reader,
        });

        // Initialize the miner, if the node type is a miner.
        if E::NODE_TYPE == NodeType::Miner && prover.pool.is_none() {
            Self::start_miner(prover.clone(), local_ip).await;
        }

        // Initialize the prover, if the node type is a prover.
        if E::NODE_TYPE == NodeType::Prover && prover.pool.is_some() {
            let prover = prover.clone();
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
                            prover.send_pool_register().await;
                        }
                    }
                }),
            );

            // Wait until the operator handler is ready.
            let _ = handler.await;
        }

        Ok(prover)
    }

    pub fn set_network_state(&self, network_state: NetworkState<N, E>) {
        self.network_state.set(network_state).expect("network state can only be set once");
    }

    fn expect_network_state(&self) -> &NetworkState<N, E> {
        self.network_state.get().expect("network state must be set")
    }

    /// Returns an instance of the memory pool.
    pub fn memory_pool(&self) -> Arc<RwLock<MemoryPool<N>>> {
        self.memory_pool.clone()
    }

    /// Returns all coinbase records in storage.
    pub fn to_coinbase_records(&self) -> Vec<(u32, Record<N>)> {
        self.state.to_coinbase_records()
    }

    ///
    /// Performs the given `request` to the prover.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub async fn update(&self, request: ProverRequest<N>) {
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
            if let Some(recipient) = self.address {
                if let Some(pool_ip) = self.pool {
                    // Proceed to register the prover to receive a block template.
                    self.expect_network_state()
                        .peers
                        .send(pool_ip, Message::PoolRegister(recipient))
                        .await;
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
            if let Some(recipient) = self.address {
                if let Some(pool_ip) = self.pool {
                    // Refuse work from any pool other than the registered one.
                    if pool_ip == operator_ip {
                        // If `terminator` is `false` and the status is not `Peering` or `Mining`
                        // already, mine the next block.
                        if !E::terminator().load(Ordering::SeqCst) && !E::status().is_peering() && !E::status().is_mining() {
                            // Set the status to `Mining`.
                            E::status().update(State::Mining);

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

                            E::status().update(State::Ready);

                            match result {
                                Ok(Ok((nonce, proof, proof_difficulty))) => {
                                    info!(
                                        "Prover successfully mined a share for unconfirmed block {} with proof difficulty of {}",
                                        block_height, proof_difficulty
                                    );

                                    // Send a `PoolResponse` to the operator.
                                    let message = Message::PoolResponse(recipient, nonce, Data::Object(proof));
                                    self.expect_network_state().peers.send(operator_ip, message).await;
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
        if let Ok(false) = self.ledger_reader.contains_transaction(&transaction.transaction_id()) {
            debug!("Adding unconfirmed transaction {} to memory pool", transaction.transaction_id());
            // Attempt to add the unconfirmed transaction to the memory pool.
            match self.memory_pool.write().await.add_transaction(&transaction) {
                Ok(()) => {
                    // Upon success, propagate the unconfirmed transaction to the connected peers.
                    self.expect_network_state()
                        .peers
                        .propagate(peer_ip, Message::UnconfirmedTransaction(Data::Object(transaction)))
                        .await;
                }
                Err(error) => error!("{}", error),
            }
        }
    }

    ///
    /// Initialize the miner, if the node type is a miner.
    ///
    async fn start_miner(prover: Arc<Self>, local_ip: SocketAddr) {
        // Initialize a new instance of the miner.
        if E::NODE_TYPE == NodeType::Miner && prover.pool.is_none() {
            if let Some(recipient) = prover.address {
                // Initialize the prover process.
                //           let prover_clone = prover.clone();
                let (router, handler) = oneshot::channel();
                E::resources().register_task(
                    None, // No need to provide an id, as the task will run indefinitely.
                    task::spawn(async move {
                        // Notify the outer function that the task is ready.
                        let _ = router.send(());

                        loop {
                            // If `terminator` is `false` and the status is not `Peering` or `Mining` already, mine the next block.
                            if !E::terminator().load(Ordering::SeqCst) && !E::status().is_peering() && !E::status().is_mining() {
                                // Set the status to `Mining`.
                                E::status().update(State::Mining);

                                // Prepare the unconfirmed transactions and dependent objects.
                                let state = prover.state.clone();
                                let canon = prover.ledger_reader.clone(); // This is *safe* as the ledger only reads.
                                let unconfirmed_transactions = prover.memory_pool.read().await.transactions();

                                // Procure a resource id to register the task with, as it might be terminated at any point in time.
                                let mining_task_id = E::resources().procure_id();
                                let prover_clone = prover.clone();
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
                                        E::status().update(State::Ready);

                                        match result {
                                            Ok(Ok((block, coinbase_record))) => {
                                                debug!("Miner has found unconfirmed block {} ({})", block.height(), block.hash());
                                                // Store the coinbase record.
                                                if let Err(error) = state.add_coinbase_record(block.height(), coinbase_record) {
                                                    warn!("[Miner] Failed to store coinbase record - {}", error);
                                                }

                                                // Broadcast the next block.
                                                prover_clone.expect_network_state().ledger.unconfirmed_block(local_ip, block).await;
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
