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

use crate::{
    helpers::{init_worker_channels, PrimaryReceiver},
    Gateway,
    Shared,
    Worker,
};
use snarkos_account::Account;
use snarkos_node_messages::Data;
use snarkvm::{
    console::prelude::*,
    prelude::{ProverSolution, PuzzleCommitment, Transaction},
};

use parking_lot::{Mutex, RwLock};
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::task::JoinHandle;

#[derive(Clone)]
pub struct Primary<N: Network> {
    /// The shared state.
    shared: Arc<Shared<N>>,
    /// The gateway.
    gateway: Gateway<N>,
    /// The workers.
    workers: Arc<RwLock<Vec<Worker<N>>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Primary<N> {
    /// Initializes a new primary instance.
    pub fn new(shared: Arc<Shared<N>>, account: Account<N>, dev: Option<u16>) -> Result<Self> {
        // Construct the gateway instance.
        let gateway = Gateway::new(shared.clone(), account, dev)?;
        // Return the primary instance.
        Ok(Self { shared, gateway, workers: Default::default(), handles: Default::default() })
    }

    /// Returns the gateway.
    pub fn gateway(&self) -> &Gateway<N> {
        &self.gateway
    }

    /// Run the primary instance.
    pub async fn run(&mut self, receiver: PrimaryReceiver<N>) -> Result<()> {
        info!("Starting the primary instance of the memory pool...");

        // Construct the worker ID.
        let id = u8::try_from(self.workers.read().len())?;
        // Construct the worker channels.
        let (tx_worker, rx_worker) = init_worker_channels();
        // Construct the worker instance.
        let mut worker = Worker::new(id, self.shared.clone(), self.gateway.clone())?;
        // Run the worker instance.
        worker.run(rx_worker).await?;
        // Add the worker to the list of workers.
        self.workers.write().push(worker);

        // Construct a map of the worker senders.
        let mut tx_workers = HashMap::new();
        // Add the worker sender to the map.
        tx_workers.insert(id, tx_worker);

        // Initialize the gateway.
        self.gateway.run(tx_workers).await?;

        // Start the primary handlers.
        self.start_handlers(receiver);

        Ok(())
    }

    /// Starts the primary handlers.
    pub fn start_handlers(&self, receiver: PrimaryReceiver<N>) {
        let PrimaryReceiver { mut rx_unconfirmed_solution, mut rx_unconfirmed_transaction } = receiver;

        // Process the unconfirmed solutions.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some(msg) = rx_unconfirmed_solution.recv().await {
                // TODO (howardwu): Choose the correct worker.
                let worker = self_clone.workers.read().first().unwrap().clone();
                // Process the unconfirmed solution.
                if let Err(e) = worker.process_unconfirmed_solution(msg).await {
                    error!("Worker {} failed process a message: {e}", worker.id());
                }
            }
        });

        // Process the unconfirmed transactions.
        let self_clone = self.clone();
        self.spawn(async move {
            while let Some(msg) = rx_unconfirmed_transaction.recv().await {
                // TODO (howardwu): Choose the correct worker.
                let worker = self_clone.workers.read().first().unwrap().clone();
                // Process the unconfirmed transaction.
                if let Err(e) = worker.process_unconfirmed_transaction(msg).await {
                    error!("Worker {} failed process a message: {e}", worker.id());
                }
            }
        });
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    pub fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the primary.
    pub async fn shut_down(&self) {
        trace!("Shutting down the primary...");
        // Iterate through the workers.
        self.workers.read().iter().for_each(|worker| {
            // Shut down the worker.
            worker.shut_down();
        });
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
        // Close the gateway.
        self.gateway.shut_down().await;
    }
}
