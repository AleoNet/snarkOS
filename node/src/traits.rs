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

use snarkos_node_messages::NodeType;
use snarkos_node_router::Routing;
use snarkvm::prelude::{Address, Network, PrivateKey, ViewKey};

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
    fn handle_signals(&self) {
        let node = self.clone();
        tokio::task::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {
                    node.shut_down().await;
                    std::process::exit(0);
                }
                Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
            }
        });
    }

    /// Shuts down the node.
    async fn shut_down(&self);
}
