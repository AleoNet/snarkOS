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

use snarkos_node_router::{messages::NodeType, Routing};
use snarkvm::prelude::{Address, Network, PrivateKey, ViewKey};

use once_cell::sync::OnceCell;
use std::sync::Arc;

#[async_trait]
pub trait NodeInterface<N: Network>: Routing<N> {
    /// Returns the node type.
    fn node_type(&self) -> NodeType {
        self.router().node_type()
    }

    /// Returns the account private key of the node.
    fn private_key(&self) -> &PrivateKey<N> {
        self.router().private_key()
    }

    /// Returns the account view key of the node.
    fn view_key(&self) -> &ViewKey<N> {
        self.router().view_key()
    }

    /// Returns the account address of the node.
    fn address(&self) -> Address<N> {
        self.router().address()
    }

    /// Returns `true` if the node is in development mode.
    fn is_dev(&self) -> bool {
        self.router().is_dev()
    }

    /// Handles OS signals for the node to intercept and perform a clean shutdown.
    /// Note: Only Ctrl-C is supported; it should work on both Unix-family systems and Windows.
    fn handle_signals() -> Arc<OnceCell<Self>> {
        // In order for the signal handler to be started as early as possible, a reference to the node needs
        // to be passed to it at a later time.
        let node: Arc<OnceCell<Self>> = Default::default();

        let node_clone = node.clone();
        tokio::task::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    if let Some(node) = node_clone.get() {
                        node.shut_down().await;
                    }
                    std::process::exit(0);
                }
                Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
            }
        });

        node
    }

    /// Shuts down the node.
    async fn shut_down(&self);
}
