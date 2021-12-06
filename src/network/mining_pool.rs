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
    helpers::{Status, Tasks},
    Environment,
    LedgerReader,
    LedgerRequest,
    LedgerRouter,
    Message,
    NodeType,
    PeersRequest,
    PeersRouter,
};
use snarkos_storage::{storage::Storage, MiningPoolState};
use snarkvm::dpc::prelude::*;

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
    /// ProposedBlock := (peer_ip, )
    ProposedBlock(SocketAddr, Block<N>),
    /// BlockHeightClear := (block_height)
    BlockHeightClear(u32),
}

///
/// A mining pool for a specific network on the node server.
///
#[derive(Debug)]
pub struct MiningPool<N: Network, E: Environment> {
    /// The state storage of the prover.
    state: Arc<MiningPoolState<N>>,
    /// The prover router of the node.
    mining_pool_router: MiningPoolRouter<N>,
    /// The status of the node.
    status: Status,
    /// The peers router of the node.
    peers_router: PeersRouter<N, E>,
    /// The ledger state of the node.
    ledger_reader: LedgerReader<N>,
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N>,
}

impl<N: Network, E: Environment> MiningPool<N, E> {
    /// Initializes a new instance of the prover.
    pub async fn open<S: Storage, P: AsRef<Path> + Copy>(
        tasks: &mut Tasks<JoinHandle<()>>,
        path: P,
        miner_address: Option<Address<N>>,
        local_ip: SocketAddr,
        status: &Status,
        peers_router: PeersRouter<N, E>,
        ledger_reader: LedgerReader<N>,
        ledger_router: LedgerRouter<N>,
    ) -> Result<Arc<Self>> {
        // // Initialize an mpsc channel for sending requests to the `Prover` struct.
        // let (prover_router, mut prover_handler) = mpsc::channel(1024);
        // // Initialize the prover pool.
        // let pool = ThreadPoolBuilder::new()
        //     .stack_size(8 * 1024 * 1024)
        //     .num_threads((num_cpus::get() / 8 * 7).max(1))
        //     .build()?;
        //
        // // Initialize the prover.
        // let prover = Arc::new(Self {
        //     state: Arc::new(ProverState::open_writer::<S, P>(path)?),
        //     miner: Arc::new(pool),
        //     prover_router,
        //     memory_pool: Arc::new(RwLock::new(MemoryPool::new())),
        //     status: status.clone(),
        //     terminator: terminator.clone(),
        //     peers_router,
        //     ledger_reader,
        //     ledger_router,
        // });
        //
        // // Initialize the handler for the prover.
        // {
        //     let prover = prover.clone();
        //     let (router, handler) = oneshot::channel();
        //     tasks.append(task::spawn(async move {
        //         // Notify the outer function that the task is ready.
        //         let _ = router.send(());
        //         // Asynchronously wait for a prover request.
        //         while let Some(request) = prover_handler.recv().await {
        //             // Hold the prover write lock briefly, to update the state of the prover.
        //             prover.update(request).await;
        //         }
        //     }));
        //     // Wait until the prover handler is ready.
        //     let _ = handler.await;
        // }
        //
        // // Initialize a new instance of the miner.
        // if E::NODE_TYPE == NodeType::Miner {
        //     if let Some(recipient) = miner {
        //         // Initialize the prover process.
        //         let prover = prover.clone();
        //         let tasks_clone = tasks.clone();
        //         let (router, handler) = oneshot::channel();
        //         tasks.append(task::spawn(async move {
        //             // Notify the outer function that the task is ready.
        //             let _ = router.send(());
        //             loop {
        //                 // If `terminator` is `false` and the status is not `Peering` or `Mining` already, mine the next block.
        //                 if !prover.terminator.load(Ordering::SeqCst) && !prover.status.is_peering() && !prover.status.is_mining() {
        //                     // Set the status to `Mining`.
        //                     prover.status.update(State::Mining);
        //
        //                     // Prepare the unconfirmed transactions, terminator, and status.
        //                     let state = prover.state.clone();
        //                     let miner = prover.miner.clone();
        //                     let canon = prover.ledger_reader.clone(); // This is *safe* as the ledger only reads.
        //                     let unconfirmed_transactions = prover.memory_pool.read().await.transactions();
        //                     let terminator = prover.terminator.clone();
        //                     let status = prover.status.clone();
        //                     let ledger_router = prover.ledger_router.clone();
        //                     let prover_router = prover.prover_router.clone();
        //
        //                     tasks_clone.append(task::spawn(async move {
        //                         // Mine the next block.
        //                         let result = task::spawn_blocking(move || {
        //                             miner.install(move || {
        //                                 canon.mine_next_block(
        //                                     recipient,
        //                                     E::COINBASE_IS_PUBLIC,
        //                                     &unconfirmed_transactions,
        //                                     &terminator,
        //                                     &mut thread_rng(),
        //                                 )
        //                             })
        //                         })
        //                         .await
        //                         .map_err(|e| e.into());
        //
        //                         // Set the status to `Ready`.
        //                         status.update(State::Ready);
        //
        //                         match result {
        //                             Ok(Ok((block, coinbase_record))) => {
        //                                 debug!("Miner has found unconfirmed block {} ({})", block.height(), block.hash());
        //                                 // Store the coinbase record.
        //                                 if let Err(error) = state.add_coinbase_record(block.height(), coinbase_record) {
        //                                     warn!("[Miner] Failed to store coinbase record - {}", error);
        //                                 }
        //
        //                                 // Broadcast the next block.
        //                                 let request = LedgerRequest::UnconfirmedBlock(local_ip, block, prover_router.clone());
        //                                 if let Err(error) = ledger_router.send(request).await {
        //                                     warn!("Failed to broadcast mined block - {}", error);
        //                                 }
        //                             }
        //                             Ok(Err(error)) | Err(error) => trace!("{}", error),
        //                         }
        //                     }));
        //                 }
        //                 // Sleep for 2 seconds.
        //                 tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        //             }
        //         }));
        //         // Wait until the miner task is ready.
        //         let _ = handler.await;
        //     } else {
        //         error!("Missing miner address. Please specify an Aleo address in order to mine");
        //     }
        // }
        // // Initialize a new instance of the mining pool.
        // else if E::NODE_TYPE == NodeType::MiningPool {
        //     // TODO (raychu86): Implement the mining pool.
        //     //  1. Send the subscribed miners the block template to mine.
        //     //  2. Track the shares sent by the miners.
        //     //  3. Broadcast valid blocks.
        //     //  4. Pay out and/or assign scores for the miners based on proportional shares or Pay-per-Share.
        // }
        //
        // Ok(prover)

        unimplemented!()
    }

    /// Returns an instance of the prover router.
    pub fn router(&self) -> MiningPoolRouter<N> {
        self.mining_pool_router.clone()
    }

    /// Returns all the shares in storage.
    pub fn to_shares(&self) -> Vec<(u32, HashMap<Address<N>, u128>)> {
        self.state.to_shares()
    }

    ///
    /// Performs the given `request` to the mining pool.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(&self, request: MiningPoolRequest<N>) {
        match request {
            MiningPoolRequest::ProposedBlock(peer_ip, block) => {
                // TODO (raychu86): Do stuff.
            }
            MiningPoolRequest::BlockHeightClear(block_height) => {
                // TODO (raychu86): Do stuff.
            }
        }
    }
}
