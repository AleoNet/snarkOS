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
    Environment,
    LedgerReader,
    LedgerRequest,
    LedgerRouter,
    NodeType,
    PeersRouter,
    ProverRouter,
};
use snarkos_storage::{storage::Storage, BlockTemplate, MiningPoolState};
use snarkvm::{algorithms::crh::sha256d_to_u64, dpc::prelude::*, utilities::ToBytes};

use anyhow::Result;
use rand::thread_rng;
use std::{collections::HashMap, net::SocketAddr, path::Path, sync::Arc};
use tokio::{
    sync::{mpsc, oneshot, RwLock},
    task,
    task::JoinHandle,
};

/// Shorthand for the parent half of the `MiningPool` message channel.
pub(crate) type MiningPoolRouter<N> = mpsc::Sender<MiningPoolRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `MiningPool` message channel.
type MiningPoolHandler<N> = mpsc::Receiver<MiningPoolRequest<N>>;

///
/// An enum of requests that the `MiningPool` struct processes.
///
#[derive(Debug)]
pub enum MiningPoolRequest<N: Network> {
    /// ProposedBlock := (peer_ip, proposed_block, miner_address)
    ProposedBlock(SocketAddr, Block<N>, Address<N>),
    /// GetCurrentBlockTemplate := (peer_ip)
    GetCurrentBlockTemplate(SocketAddr),
    /// BlockHeightClear := (block_height)
    BlockHeightClear(u32),
}

///
/// A mining pool for a specific network on the node server.
///
#[derive(Debug)]
pub struct MiningPool<N: Network, E: Environment> {
    /// The address of the mining pool.
    mining_pool_address: Option<Address<N>>,
    /// The local address of this node.
    local_ip: SocketAddr,
    /// The state storage of the mining pool.
    state: Arc<MiningPoolState<N>>,
    /// The mining pool router of the node.
    mining_pool_router: MiningPoolRouter<N>,
    /// The status of the node.
    status: Status,
    /// The pool of uncer: PeersRouter<N, E>,onfirmed transactions.
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
}

impl<N: Network, E: Environment> MiningPool<N, E> {
    /// Initializes a new instance of the mining pool.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        tasks: &mut Tasks<JoinHandle<()>>,
        path: P,
        mining_pool_address: Option<Address<N>>,
        local_ip: SocketAddr,
        status: Status,
        memory_pool: Arc<RwLock<MemoryPool<N>>>,
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
        prover_router: ProverRouter<N>,
    ) -> Result<Arc<Self>> {
        // Initialize an mpsc channel for sending requests to the `MiningPool` struct.
        let (mining_pool_router, mut mining_pool_handler) = mpsc::channel(1024);

        // Initialize the mining pool.
        let mining_pool = Arc::new(Self {
            mining_pool_address,
            local_ip,
            state: Arc::new(MiningPoolState::open_writer::<S, P>(path)?),
            mining_pool_router,
            status: status.clone(),
            memory_pool,
            peers_router,
            ledger_reader,
            ledger_router,
            prover_router,
            current_template: RwLock::new(None),
        });

        if E::NODE_TYPE == NodeType::MiningPool {
            // Initialize the handler for the mining pool.
            {
                let mining_pool = mining_pool.clone();
                let (router, handler) = oneshot::channel();
                tasks.append(task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // Asynchronously wait for a mining pool request.
                    while let Some(request) = mining_pool_handler.recv().await {
                        mining_pool.update(request).await;
                    }
                }));
                // Wait until the mining pool handler is ready.
                let _ = handler.await;
            }

            // TODO (raychu86): Implement the mining pool.
            //  1. Send the subscribed miners the block template to mine.
            //  2. Track the shares sent by the miners.
            //  3. Broadcast valid blocks.
            //  4. Pay out and/or assign scores for the miners based on proportional shares or Pay-per-Share.

            if let Some(recipient) = mining_pool_address {
                // Set initial block template.
                mining_pool.set_block_template(recipient).await?;

                // Initialize the mining pool process.
                let mining_pool = mining_pool.clone();
                let tasks_clone = tasks.clone();
                let (router, handler) = oneshot::channel();
                tasks.append(task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    loop {
                        // If the status is not `Peering` or `Mining` already, mine the next block.
                        if !mining_pool.status.is_peering() && !mining_pool.status.is_mining() {
                            // Set the status to `Mining`.
                            mining_pool.status.update(State::Mining);

                            // Send block templates to the miners.
                            tasks_clone.append(task::spawn(async move {
                                // TODO (raychu86): Send a block template to the subscribed peers
                                //  whenever the canon chain advances.
                            }));

                            // Set the status to `Ready`.
                            status.update(State::Ready);
                        }
                        // Sleep for 2 seconds.
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }));
                // Wait until the miner task is ready.
                let _ = handler.await;
            } else {
                error!("Missing miner address. Please specify an Aleo address in order to run a mining pool");
            }
        }

        Ok(mining_pool)
    }

    /// Returns an instance of the mining pool router.
    pub fn router(&self) -> MiningPoolRouter<N> {
        self.mining_pool_router.clone()
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u64>)> {
        self.state.to_shares()
    }

    ///
    /// Performs the given `request` to the mining pool.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: MiningPoolRequest<N>) {
        match request {
            MiningPoolRequest::ProposedBlock(peer_ip, block, miner_address) => {
                // Check that the block is relevant.
                if self.ledger_reader.latest_block_height().saturating_add(1) != block.height() {
                    warn!("[ProposedBlock] Peer {} sent a stale candidate block.", peer_ip);
                    return;
                }

                // TODO (raychu86): Check that the block is valid except for the block difficulty.

                // Check that the block's coinbase transaction owner is the mining pool address.
                match block.to_coinbase_transaction() {
                    Ok(tx) => {
                        let coinbase_records: Vec<Record<N>> = tx.to_records().collect();
                        let valid_owner = coinbase_records
                            .iter()
                            .map(|r| Some(r.owner()) == self.mining_pool_address)
                            .fold(false, |a, b| a || b);

                        if !valid_owner {
                            warn!("[ProposedBlock] Peer {} sent a candidate block with an invalid owner.", peer_ip);
                            return;
                        }
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
                let shares = u64::MAX / hash_difficulty;

                // Update the score for the miner.
                if let Err(error) = self.state.add_shares(block.height(), &miner_address, shares) {
                    warn!("[ProposedBlock] {}", error);
                }

                // If the block is valid, broadcast it.
                if block.is_valid() {
                    debug!("Mining pool has found unconfirmed block {} ({})", block.height(), block.hash());
                    // TODO (raychu86): Store the coinbase record.

                    // Broadcast the next block.
                    let request = LedgerRequest::UnconfirmedBlock(self.local_ip, block, self.prover_router.clone());
                    if let Err(error) = self.ledger_router.send(request).await {
                        warn!("Failed to broadcast mined block - {}", error);
                    }
                }
            }
            MiningPoolRequest::BlockHeightClear(block_height) => {
                // Remove the shares for the given block height.
                if let Err(error) = self.state.remove_shares(block_height) {
                    warn!("[BlockHeightClear] {}", error);
                }
            }
        }
    }

    async fn set_block_template(&self, recipient: Address<N>) -> Result<()> {
        let unconfirmed_transactions = self.memory_pool.read().await.transactions();
        let mut current_template = self.current_template.write().await;
        let (mut block_template, _) =
            self.ledger_reader
                .prepare_block_template(recipient, E::COINBASE_IS_PUBLIC, &unconfirmed_transactions, &mut thread_rng())?;

        // TODO: Ensure the difficulty target is low enough for miners to produce valid shares.

        *current_template = Some(block_template);
        Ok(())
    }
}
