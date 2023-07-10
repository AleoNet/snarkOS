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
    helpers::{PrimaryReceiver, PrimarySender, Storage},
    Primary,
};
use snarkos_account::Account;
use snarkvm::{
    console::account::Address,
    prelude::{Network, Result},
};

#[derive(Clone)]
pub struct Bullshark<N: Network> {
    /// The primary.
    primary: Primary<N>,
    /// The leader of the previous round, if one was present.
    previous_leader: Option<Address<N>>,
}

impl<N: Network> Bullshark<N> {
    /// Initializes a new instance of Bullshark.
    pub fn new(storage: Storage<N>, account: Account<N>, dev: Option<u16>) -> Result<Self> {
        Ok(Self { primary: Primary::new(storage, account, dev)?, previous_leader: None })
    }

    /// Run the Bullshark instance.
    pub async fn run(&mut self, sender: PrimarySender<N>, receiver: PrimaryReceiver<N>) -> Result<()> {
        self.primary.run(sender, receiver).await
    }

    /// Returns the primary.
    pub const fn primary(&self) -> &Primary<N> {
        &self.primary
    }

    /// Returns the previous round leader, if one was present.
    pub const fn previous_leader(&self) -> Option<Address<N>> {
        self.previous_leader
    }
}
