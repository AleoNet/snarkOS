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

use crate::{helpers::GatewayReceiver, Gateway, Shared, Worker};
use snarkvm::console::prelude::*;

use std::sync::Arc;

pub struct Primary<N: Network> {
    /// The shared state.
    shared: Arc<Shared<N>>,
    /// The gateway.
    gateway: Gateway<N>,
    /// The gateway receiver, which allows the router to forward messages to the gateway.
    gateway_receiver: GatewayReceiver<N>,
    /// The workers.
    workers: Vec<Worker<N>>,
}

impl<N: Network> Primary<N> {
    /// Initializes a new primary instance.
    pub fn new(shared: Arc<Shared<N>>, gateway_receiver: GatewayReceiver<N>) -> Result<Self> {
        // Construct the gateway instance.
        let gateway = Gateway::new(shared.clone())?;
        // Return the primary instance.
        Ok(Self { shared, gateway, gateway_receiver, workers: Vec::new() })
    }

    /// Run the primary instance.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting the primary instance of the memory pool");

        // Construct the worker ID.
        let id = u8::try_from(self.workers.len())?;
        // Construct the worker instance.
        let worker = Worker::new(id, self.shared.clone(), self.gateway.clone())?;
        // Add the worker to the list of workers.
        self.workers.push(worker);

        // // the task for processing parsed messages
        // let self_clone = self.clone();
        // let inbound_processing_task = tokio::spawn(async move {
        //     let node = self_clone.tcp();
        //     trace!(parent: node.span(), "spawned a task for processing messages from {}", addr);
        //     tx_processing.send(()).unwrap(); // safe; the channel was just opened
        //
        //     while let Some(msg) = self.gateway_receiver.rx_unconfirmed_solution.recv().await {
        //         if let Err(e) = self_clone.process_message(addr, msg).await {
        //             error!(parent: node.span(), "can't process a message from {}: {}", addr, e);
        //             node.known_peers().register_failure(addr);
        //         }
        //     }
        // });

        Ok(())
    }
}
