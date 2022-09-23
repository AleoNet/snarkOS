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

use snarkvm::{compiler::CoinbasePuzzle, prelude::*};

use crate::{
    environment::{
        helpers::{NodeType, Status},
        Environment,
    },
    network::Message,
    Data,
    Ledger,
};

use std::{
    marker::PhantomData,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

/// Shorthand for the parent half of the `Prover` message channel.
pub type ProverRouter = mpsc::Sender<ProverRequest>;
/// Shorthand for the child half of the `Prover` message channel.
pub type ProverHandler = mpsc::Receiver<ProverRequest>;

// TODO (raychu86): Add prover functionality.
/// An enum of requests that the `Prover` struct processes.
pub enum ProverRequest {}

/// The prover heartbeat in milliseconds.
const PROVER_HEARTBEAT_IN_MILLISECONDS: Duration = Duration::from_millis(500);

pub struct Prover<N: Network, E: Environment> {
    /// The prover router of the node.
    router: ProverRouter,
    /// The shared ledger state of the owned node.
    ledger: Arc<Ledger<N>>,
    /// PhantomData.
    _phantom: PhantomData<(N, E)>,
}

impl<N: Network, E: Environment> Prover<N, E> {
    /// Initialize a new instance of hte prover, paired with its handler.
    pub fn new(ledger: Arc<Ledger<N>>) -> Result<(Self, ProverHandler)> {
        // Initialize an mpsc channel for sending requests to the `Prover` struct.
        let (router, handler) = mpsc::channel(1024);

        // Initialize the prover.
        let prover = Self {
            router,
            ledger,
            _phantom: PhantomData,
        };

        Ok((prover, handler))
    }

    // TODO (raychu86): This operation is done independently. Need to evaluate if the provers should be
    //  requesting epoch state from the validators, or continue with the latest prover state.
    /// Starts the prover and sends coinbase proofs to the validators.
    pub async fn start_prover(&self) {
        if E::NODE_TYPE == NodeType::Prover {
            // Initialize the prover process.
            let (router, handler) = oneshot::channel();
            let ledger = self.ledger.clone();
            E::resources().register_task(
                None,
                tokio::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());

                    loop {
                        // If `terminator` is `false` and the status is not `Peering` already,
                        // then generate a coinbase proof.

                        if !E::terminator().load(Ordering::SeqCst) & E::status().is_ready() {
                            // Set the status to `Proving`.
                            E::status().update(Status::Proving);

                            let ledger = ledger.clone();

                            // Craft a coinbase proof.
                            let proving_task_id = E::resources().procure_id();
                            E::resources().register_task(
                                Some(proving_task_id),
                                tokio::spawn(async move {
                                    // Construct a coinbase proof.
                                    let epoch_info = ledger.ledger().read().latest_epoch_info();

                                    trace!("Generating proof for round {}", epoch_info.epoch_number);

                                    let epoch_challenge = match ledger.ledger().read().latest_epoch_challenge() {
                                        Ok(challenge) => challenge,
                                        Err(error) => {
                                            warn!("Failed to get epoch challenge: {}", error);
                                            return;
                                        }
                                    };

                                    let nonce = u64::rand(&mut ::rand::thread_rng());

                                    let prover_solution = match CoinbasePuzzle::<N>::prove(
                                        ledger.ledger().read().coinbase_puzzle_proving_key(),
                                        &epoch_info,
                                        &epoch_challenge,
                                        ledger.address(),
                                        nonce,
                                    ) {
                                        Ok(proof) => proof,
                                        Err(error) => {
                                            warn!("Failed to generate prover solution: {}", error);
                                            return;
                                        }
                                    };

                                    // Fetch the prover solution difficulty target.
                                    let prover_solution_difficulty_target = match prover_solution.to_difficulty_target() {
                                        Ok(difficulty_target) => difficulty_target,
                                        Err(error) => {
                                            warn!("Failed to fetch prover solution difficulty target: {}", error);
                                            return;
                                        }
                                    };

                                    // Fetch the latest proof target.
                                    let proof_target = match ledger.ledger().read().latest_proof_target() {
                                        Ok(target) => target,
                                        Err(error) => {
                                            warn!("Failed to get latest proof target: {}", error);
                                            return;
                                        }
                                    };

                                    // Ensure that the prover solution difficulty is sufficient.
                                    if prover_solution_difficulty_target < proof_target {
                                        warn!("Generated coinbase proof does not meet the target difficulty");
                                        return;
                                    }

                                    // Send the coinbase proof to the peers.
                                    let peers = ledger.peers().read().clone();

                                    for (socket_addr, _) in peers.iter() {
                                        match peers.get(socket_addr) {
                                            Some(sender) => {
                                                let _ = sender.send(Message::<N>::ProverSolution(Data::Object(prover_solution))).await;
                                            }
                                            None => {
                                                warn!("Error finding validator '{}' in peers list", socket_addr);
                                            }
                                        }
                                    }

                                    // Set the status to `Ready`.
                                    E::status().update(Status::Ready);

                                    // Deregister the proving task id.
                                    E::resources().deregister(proving_task_id);
                                }),
                            );
                        }

                        // Proceed to sleep for a preset amount of time
                        tokio::time::sleep(PROVER_HEARTBEAT_IN_MILLISECONDS).await;
                    }
                }),
            );

            // Wait until the prover process is ready
            let _ = handler.await;
        }
    }

    ///
    /// Performs the given `request` to the prover.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub async fn update(&self, request: ProverRequest) {
        match request {}
    }

    /// Returns an instance of the prover router.
    pub fn router(&self) -> &ProverRouter {
        &self.router
    }
}
