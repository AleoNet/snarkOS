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

use crate::{Gateway, Shared};
use snarkvm::console::prelude::*;

use std::sync::Arc;

pub struct Worker<N: Network> {
    /// The worker ID.
    id: u8,
    /// The shared state.
    shared: Arc<Shared<N>>,
    /// The gateway.
    gateway: Gateway<N>,
}

impl<N: Network> Worker<N> {
    /// Initializes a new worker instance.
    pub fn new(id: u8, shared: Arc<Shared<N>>, gateway: Gateway<N>) -> Result<Self> {
        // Return the worker.
        Ok(Self { id, shared, gateway })
    }

    /// Run the worker instance.
    pub async fn run(&mut self) -> Result<(), Error> {
        info!("Starting worker instance {} of the memory pool", self.id);

        // // Create the validator instance.
        // let mut validator = Validator::new(self.shared.clone());
        //
        // // Run the validator instance.
        // validator.run().await?;

        Ok(())
    }
}
