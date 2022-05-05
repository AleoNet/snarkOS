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

use std::{net::SocketAddr, sync::Arc};

use once_cell::race::OnceBox;
use snarkos_environment::{helpers::NodeType, Environment};
use snarkvm::prelude::*;
use tokio::sync::oneshot;

use crate::{
    ledger::{Ledger, LedgerHandler},
    operator::{Operator, OperatorHandler},
    peers::{Peers, PeersHandler},
    prover::{Prover, ProverHandler},
};

pub struct State<N: Network, E: Environment> {
    /// The local address of the node.
    pub local_ip: SocketAddr,
    /// The Aleo address corresponding to the Node's prover and/or operator.
    pub address: Option<Address<N>>,
    /// The list of peers for the node.
    peers: OnceBox<Peers<N, E>>,
    /// The ledger of the node.
    ledger: OnceBox<Ledger<N, E>>,
    /// The prover of the node.
    prover: OnceBox<Prover<N, E>>,
    /// The operator of the node.
    operator: OnceBox<Operator<N, E>>,
}

impl<N: Network, E: Environment> State<N, E> {
    pub fn new(local_ip: SocketAddr, address: Option<Address<N>>) -> Self {
        Self {
            local_ip,
            address,
            peers: Default::default(),
            ledger: Default::default(),
            operator: Default::default(),
            prover: Default::default(),
        }
    }

    pub async fn initialize_peers(self: &Arc<Self>, peers: Peers<N, E>, mut peers_handler: PeersHandler<N, E>) {
        self.peers.set(peers.into()).map_err(|_| ()).unwrap();

        let state = self.clone();
        let (router, handler) = oneshot::channel();
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            tokio::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a peers request.
                while let Some(request) = peers_handler.recv().await {
                    let state = state.clone();
                    // Procure a resource id to register the task with, as it might be terminated at any point in time.
                    let resource_id = E::resources().procure_id();
                    // Asynchronously process a peers request.
                    E::resources().register_task(
                        Some(resource_id),
                        tokio::spawn(async move {
                            // Update the state of the peers.
                            state.peers().update(request).await;

                            E::resources().deregister(resource_id);
                        }),
                    );
                }
            }),
        );

        // Wait until the peers router task is ready.
        let _ = handler.await;
    }

    pub async fn initialize_ledger(self: &Arc<Self>, ledger: Ledger<N, E>, mut ledger_handler: LedgerHandler<N>) {
        self.ledger.set(ledger.into()).map_err(|_| ()).unwrap();

        let state = self.clone();
        let (router, handler) = oneshot::channel();
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            tokio::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a ledger request.
                while let Some(request) = ledger_handler.recv().await {
                    // Update the state of the ledger.
                    // Note: Do not wrap this call in a `tokio::spawn` as `BlockResponse` messages
                    // will end up being processed out of order.
                    state.ledger().update(request).await;
                }
            }),
        );

        // Wait until the ledger handler is ready.
        let _ = handler.await;
    }

    pub async fn initialize_prover(self: &Arc<Self>, prover: Prover<N, E>, mut prover_handler: ProverHandler<N>) {
        self.prover.set(prover.into()).map_err(|_| ()).unwrap();

        let state = self.clone();
        let (router, handler) = oneshot::channel();
        E::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            tokio::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                // Asynchronously wait for a prover request.
                while let Some(request) = prover_handler.recv().await {
                    // Update the state of the prover.
                    state.prover().update(request).await;
                }
            }),
        );

        // Wait until the prover handler is ready.
        let _ = handler.await;
    }

    pub async fn initialize_operator(self: &Arc<Self>, operator: Operator<N, E>, mut operator_handler: OperatorHandler<N>) {
        self.operator.set(operator.into()).map_err(|_| ()).unwrap();

        if E::NODE_TYPE == NodeType::Operator {
            // Initialize the handler for the operator.
            let state = self.clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None, // No need to provide an id, as the task will run indefinitely.
                tokio::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // Asynchronously wait for a operator request.
                    while let Some(request) = operator_handler.recv().await {
                        state.operator().update(request).await;
                    }
                }),
            );

            // Wait until the operator handler is ready.
            let _ = handler.await;
        }
    }

    pub fn peers(&self) -> &Peers<N, E> {
        self.peers.get().unwrap()
    }

    pub fn ledger(&self) -> &Ledger<N, E> {
        self.ledger.get().unwrap()
    }

    pub fn prover(&self) -> &Prover<N, E> {
        self.prover.get().unwrap()
    }

    pub fn operator(&self) -> &Operator<N, E> {
        self.operator.get().unwrap()
    }
}
