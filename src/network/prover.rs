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
    helpers::{State, Status},
    Environment,
    LedgerReader,
    LedgerRequest,
    LedgerRouter,
    Message,
    NodeType,
    PeersRequest,
    PeersRouter,
};
use snarkvm::dpc::prelude::*;

use anyhow::Result;
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
    sync::{mpsc, RwLock},
    task,
};

/// Shorthand for the parent half of the `Prover` message channel.
pub(crate) type ProverRouter<N> = mpsc::Sender<ProverRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Prover` message channel.
type ProverHandler<N> = mpsc::Receiver<ProverRequest<N>>;

///
/// An enum of requests that the `Prover` struct processes.
///
#[derive(Debug)]
pub enum ProverRequest<N: Network> {
    /// Mine := (local_ip, prover_address)
    Mine(SocketAddr, Address<N>),
    /// UnconfirmedTransaction := (peer_ip, transaction)
    UnconfirmedTransaction(SocketAddr, Transaction<N>),
}

///
/// A prover for a specific network on the node server.
///
#[derive(Debug)]
#[allow(clippy::type_complexity)]
pub struct Prover<N: Network, E: Environment> {
    /// The thread pool for the prover.
    prover: Arc<ThreadPool>,
    /// The pool of unconfirmed transactions.
    memory_pool: RwLock<MemoryPool<N>>,
    /// The status of the node.
    status: Status,
    /// A terminator bit for the prover.
    terminator: Arc<AtomicBool>,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The ledger state of the node.
    ledger_reader: LedgerReader<N>,
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N, E>,
}

impl<N: Network, E: Environment> Prover<N, E> {
    /// Initializes a new instance of the prover.
    pub fn open(
        status: &Status,
        terminator: &Arc<AtomicBool>,
        peers_router: &PeersRouter<N, E>,
        ledger_reader: &LedgerReader<N>,
        ledger_router: &LedgerRouter<N, E>,
    ) -> Result<Self> {
        Ok(Self {
            prover: Arc::new(ThreadPoolBuilder::new().num_threads((num_cpus::get() / 8 * 2).max(1)).build()?),
            memory_pool: RwLock::new(MemoryPool::new()),
            status: status.clone(),
            terminator: terminator.clone(),
            peers_router: peers_router.clone(),
            ledger_reader: ledger_reader.clone(),
            ledger_router: ledger_router.clone(),
        })
    }

    ///
    /// Performs the given `request` to the prover.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: ProverRequest<N>) {
        match request {
            ProverRequest::Mine(local_ip, recipient) => {
                // Process the request to mine the next block.
                self.mine_next_block(local_ip, recipient).await;
            }
            ProverRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Ensure the node is not peering.
                if !self.status.is_peering() {
                    // Process the unconfirmed transaction.
                    self.add_unconfirmed_transaction(peer_ip, transaction).await
                }
            }
        }
    }

    ///
    /// Mines a new block and adds it to the canon blocks.
    ///
    async fn mine_next_block(&self, local_ip: SocketAddr, recipient: Address<N>) {
        // If the node type is not a miner, it should not be mining.
        if E::NODE_TYPE != NodeType::Miner {
            return;
        }
        // If `terminator` is `true`, it should not be mining.
        if self.terminator.load(Ordering::SeqCst) {
            return;
        }
        // If the status is `Ready`, mine the next block.
        if self.status.is_ready() {
            // Set the status to `Mining`.
            self.status.update(State::Mining);

            // Prepare the unconfirmed transactions, terminator, and status.
            let prover = self.prover.clone();
            let canon = self.ledger_reader.clone(); // This is *safe* as the ledger only reads.
            let unconfirmed_transactions = self.memory_pool.read().await.transactions();
            let terminator = self.terminator.clone();
            let status = self.status.clone();
            let ledger_router = self.ledger_router.clone();

            task::spawn(async move {
                // Mine the next block.
                let result = task::spawn_blocking(move || {
                    prover.install(move || canon.mine_next_block(recipient, &unconfirmed_transactions, &terminator, &mut thread_rng()))
                })
                .await
                .map_err(|e| e.into());

                // Set the status to `Ready`.
                status.update(State::Ready);

                match result {
                    Ok(Ok(block)) => {
                        debug!("Miner has found an unconfirmed candidate for block {}", block.height());
                        // Broadcast the next block.
                        let request = LedgerRequest::UnconfirmedBlock(local_ip, block);
                        if let Err(error) = ledger_router.send(request).await {
                            warn!("Failed to broadcast mined block: {}", error);
                        }
                    }
                    Ok(Err(error)) | Err(error) => trace!("{}", error),
                }
            });
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
                    let request = PeersRequest::MessagePropagate(peer_ip, Message::UnconfirmedTransaction(transaction));
                    if let Err(error) = self.peers_router.send(request).await {
                        warn!("[UnconfirmedTransaction] {}", error);
                    }
                }
                Err(error) => error!("{}", error),
            }
        }
    }
}
