// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod router;

use crate::traits::NodeInterface;
use snarkos_account::Account;
use snarkos_node_bft::{ledger_service::CoreLedgerService, MAX_TRANSMISSIONS_PER_BATCH};
use snarkos_node_rest::Rest;
use snarkos_node_router::{
    messages::{Message, NodeType, UnconfirmedSolution, UnconfirmedTransaction},
    Heartbeat,
    Inbound,
    Outbound,
    Router,
    Routing,
};
use snarkos_node_sync::{BlockSync, BlockSyncMode};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    P2P,
};
use snarkvm::{
    console::network::Network,
    ledger::{
        block::{Block, Header},
        coinbase::{CoinbasePuzzle, EpochChallenge, ProverSolution},
        store::ConsensusStorage,
        Ledger,
    },
    prelude::block::Transaction,
};

use anyhow::Result;
use core::future::Future;
use lru::LruCache;
use parking_lot::Mutex;
use std::{
    net::SocketAddr,
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed},
        Arc,
    },
    time::Duration,
};
use tokio::{task::JoinHandle, time::sleep};

const VERIFICATION_CONCURRENCY_LIMIT: usize = 6; // 8 deployments of MAX_NUM_CONSTRAINTS will run out of memory.

/// Transaction details needed for propagation.
/// We preserve the serialized transaction for faster propagation.
type TransactionContents<N> = (SocketAddr, UnconfirmedTransaction<N>, Transaction<N>);

/// A client node is a full node, capable of querying with the network.
#[derive(Clone)]
pub struct Client<N: Network, C: ConsensusStorage<N>> {
    /// The ledger of the node.
    ledger: Ledger<N, C>,
    /// The router of the node.
    router: Router<N>,
    /// The REST server of the node.
    rest: Option<Rest<N, C, Self>>,
    /// The sync module.
    sync: Arc<BlockSync<N>>,
    /// The genesis block.
    genesis: Block<N>,
    /// The coinbase puzzle.
    coinbase_puzzle: CoinbasePuzzle<N>,
    /// The unconfirmed transactions queue.
    transaction_queue: Arc<Mutex<LruCache<N::TransactionID, TransactionContents<N>>>>,
    /// The amount of transactions currently being verified.
    verification_counter: Arc<AtomicUsize>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
}

impl<N: Network, C: ConsensusStorage<N>> Client<N, C> {
    /// Initializes a new client node.
    pub async fn new(
        node_ip: SocketAddr,
        rest_ip: Option<SocketAddr>,
        rest_rps: u32,
        account: Account<N>,
        trusted_peers: &[SocketAddr],
        genesis: Block<N>,
        cdn: Option<String>,
        dev: Option<u16>,
    ) -> Result<Self> {
        // Prepare the shutdown flag.
        let shutdown: Arc<AtomicBool> = Default::default();

        // Initialize the signal handler.
        let signal_node = Self::handle_signals(shutdown.clone());

        // Initialize the ledger.
        let ledger = Ledger::<N, C>::load(genesis.clone(), dev.into())?;
        // TODO: Remove me after Phase 3.
        let ledger = crate::phase_3_reset(ledger, dev)?;
        // Initialize the CDN.
        if let Some(base_url) = cdn {
            // Sync the ledger with the CDN.
            if let Err((_, error)) =
                snarkos_node_cdn::sync_ledger_with_cdn(&base_url, ledger.clone(), shutdown.clone()).await
            {
                crate::log_clean_error(dev);
                return Err(error);
            }
        }

        // Initialize the ledger service.
        let ledger_service = Arc::new(CoreLedgerService::<N, C>::new(ledger.clone(), shutdown.clone()));
        // Initialize the sync module.
        let sync = BlockSync::new(BlockSyncMode::Router, ledger_service.clone());

        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Client,
            account,
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            dev.is_some(),
        )
        .await?;
        // Load the coinbase puzzle.
        let coinbase_puzzle = CoinbasePuzzle::<N>::load()?;
        // Initialize the node.
        let mut node = Self {
            ledger: ledger.clone(),
            router,
            rest: None,
            sync: Arc::new(sync),
            genesis,
            coinbase_puzzle,
            transaction_queue: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(MAX_TRANSMISSIONS_PER_BATCH).unwrap(),
            ))),
            verification_counter: Default::default(),
            handles: Default::default(),
            shutdown,
        };

        // Initialize the REST server.
        if let Some(rest_ip) = rest_ip {
            node.rest = Some(Rest::start(rest_ip, rest_rps, None, ledger.clone(), Arc::new(node.clone())).await?);
        }
        // Initialize the routing.
        node.initialize_routing().await;
        // Initialize the sync module.
        node.initialize_sync();
        // Initialize transaction verification.
        node.initialize_transaction_verification();
        // Initialize the notification message loop.
        node.handles.lock().push(crate::start_notification_message_loop());
        // Pass the node to the signal handler.
        let _ = signal_node.set(node.clone());
        // Return the node.
        Ok(node)
    }

    /// Returns the ledger.
    pub fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
    }

    /// Returns the REST server.
    pub fn rest(&self) -> &Option<Rest<N, C, Self>> {
        &self.rest
    }
}

impl<N: Network, C: ConsensusStorage<N>> Client<N, C> {
    /// Initializes the sync pool.
    fn initialize_sync(&self) {
        // Start the sync loop.
        let node = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if node.shutdown.load(Relaxed) {
                    info!("Shutting down block production");
                    break;
                }

                // Sleep briefly to avoid triggering spam detection.
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                // Perform the sync routine.
                node.sync.try_block_sync(&node).await;
            }
        }));
    }

    /// Initializes transaction verification.
    fn initialize_transaction_verification(&self) {
        // Start the transaction verification loop.
        let node = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if node.shutdown.load(Relaxed) {
                    info!("Shutting down transaction verification");
                    break;
                }

                // Determine if the queue contains txs to verify.
                let queue_is_empty = node.transaction_queue.lock().is_empty();
                // Determine if our verification counter has space to verify new txs.
                let counter_is_full = node.verification_counter.load(Relaxed) >= VERIFICATION_CONCURRENCY_LIMIT;

                // Sleep to allow the queue to be filled or transactions to be validated.
                if queue_is_empty || counter_is_full {
                    sleep(Duration::from_millis(50)).await;
                    continue;
                }

                // Determine how many transactions we want to verify.
                let queue_len = node.transaction_queue.lock().len();
                let num_transactions = queue_len.min(VERIFICATION_CONCURRENCY_LIMIT);

                // Check if we have room to verify the transactions and update the counter accordingly.
                let previous_verification_counter = node.verification_counter.fetch_update(Relaxed, Relaxed, |c| {
                    // If we are still verifying sufficient transactions, don't verify any more for now.
                    if c >= VERIFICATION_CONCURRENCY_LIMIT {
                        None
                    // If we have space to verify more transactions, verify as many as we can.
                    } else {
                        // Consider verifying all desired txs, but limit to the concurrency limit.
                        Some((c + num_transactions).min(VERIFICATION_CONCURRENCY_LIMIT))
                    }
                });

                // Determine how many transactions we cán verify.
                let num_transactions = match previous_verification_counter {
                    // Determine how many transactions we can verify.
                    Ok(previous_value) => num_transactions.saturating_sub(previous_value),
                    // If we are already verifying sufficient transactions, don't verify any more for now.
                    Err(_) => continue,
                };

                // For each transaction, spawn a task to verify it.
                let mut tx_queue = node.transaction_queue.lock();
                for _ in 0..num_transactions {
                    if let Some((_, (peer_ip, serialized, transaction))) = tx_queue.pop_lru() {
                        let _node = node.clone();
                        tokio::spawn(async move {
                            // Check the transaction.
                            if _node.ledger.check_transaction_basic(&transaction, None, &mut rand::thread_rng()).is_ok()
                            {
                                // Propagate the `UnconfirmedTransaction`.
                                _node.propagate(Message::UnconfirmedTransaction(serialized), &[peer_ip]);
                            }
                            // Reduce the verification counter.
                            _node.verification_counter.fetch_sub(1, Relaxed);
                        });
                    }
                }
            }
        }));
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    pub fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Client<N, C> {
    /// Shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");

        // Shut down the node.
        trace!("Shutting down the node...");
        self.shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

        // Abort the tasks.
        trace!("Shutting down the validator...");
        self.handles.lock().iter().for_each(|handle| handle.abort());

        // Shut down the router.
        self.router.shut_down().await;

        info!("Node has shut down.");
    }
}
