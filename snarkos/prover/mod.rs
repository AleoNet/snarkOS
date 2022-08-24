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

use snarkvm::prelude::*;

use crate::{
    environment::{
        helpers::{NodeType, Status},
        Environment,
    },
    Node,
};

use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

/// Shorthand for the parent half of the `Prover` message channel.
pub type ProverRouter = mpsc::Sender<ProverRequest>;
/// Shorthand for the child half of the `Prover` message channel.
pub type ProverHandler = mpsc::Receiver<ProverRequest>;

/// An enum of requests that the `Prover` struct processes.
pub enum ProverRequest {}

/// The prover heartbeat in seconds.
const PROVER_HEARTBEAT_IN_SECONDS: Duration = Duration::from_secs(1);

pub struct Prover<N: Network, E: Environment> {
    /// The prover router of the node.
    router: ProverRouter,
    /// The shared ledger state of the owned node.
    node: Arc<Node<N, E>>,
}

impl<N: Network, E: Environment> Prover<N, E> {
    /// Initialize a new instance of hte prover, paired with its handler.
    async fn new(node: Arc<Node<N, E>>) -> Result<(Self, mpsc::Receiver<ProverRequest>)> {
        // Initialize an mpsc channel for sending requests to the `Prover` struct.
        let (router, sender) = mpsc::channel(1024);

        // Initialize the prover.
        let prover = Self { router, node };

        Ok((prover, sender))
    }

    // TODO (raychu86): This operation is done independently. Need to evaluate if the provers should be
    //  requesting epoch state from the validators, or continue with the latest prover state.
    /// Start the prover operations.
    async fn start_prover(&self) {
        if E::NODE_TYPE == NodeType::Prover {
            // Initialize the prover process.
            let (router, handler) = oneshot::channel();
            let node = self.node.clone();
            E::resources().register_task(
                None,
                tokio::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());

                    loop {
                        // If `terminator` is `false` and the status is not `Peering` already,
                        // then generate a coinbase proof.
                        if E::terminator().load(Ordering::SeqCst) & !E::status().is_peering() {
                            // Set the status to `Proving`.
                            E::status().update(Status::Proving);

                            let _node = node.clone();

                            // Craft a coinbase proof.
                            let proving_task_id = E::resources().procure_id();
                            E::resources().register_task(
                                Some(proving_task_id),
                                tokio::spawn(async move {
                                    // Construct a coinbase proof.
                                    let coinbase_proof = ();

                                    // Send the coinbase proof to the validators.
                                    // TODO (raychu86): Implement this.

                                    // Set the status to `Ready`.
                                    E::status().update(Status::Ready);

                                    // Deregister the proving task id.
                                    E::resources().deregister(proving_task_id);
                                }),
                            );
                        }

                        // Proceed to sleep for a preset amount of time
                        tokio::time::sleep(PROVER_HEARTBEAT_IN_SECONDS).await;
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
    async fn update(&self, _request: ProverRequest) -> Result<()> {
        Ok(())
    }

    /// Returns an instance of the prover router.
    pub fn router(&self) -> &ProverRouter {
        &self.router
    }
}
