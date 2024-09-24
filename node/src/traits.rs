// Copyright 2024 Aleo Network Foundation
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
use std::{
    future::Future,
    io,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

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
    /// The optional `shutdown_flag` flag can be used to cleanly terminate the syncing process.
    fn handle_signals(shutdown_flag: Arc<AtomicBool>) -> Arc<OnceCell<Self>> {
        // In order for the signal handler to be started as early as possible, a reference to the node needs
        // to be passed to it at a later time.
        let node: Arc<OnceCell<Self>> = Default::default();

        #[cfg(target_family = "unix")]
        fn signal_listener() -> impl Future<Output = io::Result<()>> {
            use tokio::signal::unix::{signal, SignalKind};

            // Handle SIGINT, SIGTERM, SIGQUIT, and SIGHUP.
            let mut s_int = signal(SignalKind::interrupt()).unwrap();
            let mut s_term = signal(SignalKind::terminate()).unwrap();
            let mut s_quit = signal(SignalKind::quit()).unwrap();
            let mut s_hup = signal(SignalKind::hangup()).unwrap();

            // Return when any of the signals above is received.
            async move {
                tokio::select!(
                    _ = s_int.recv() => (),
                    _ = s_term.recv() => (),
                    _ = s_quit.recv() => (),
                    _ = s_hup.recv() => (),
                );
                Ok(())
            }
        }
        #[cfg(not(target_family = "unix"))]
        fn signal_listener() -> impl Future<Output = io::Result<()>> {
            tokio::signal::ctrl_c()
        }

        let node_clone = node.clone();
        tokio::task::spawn(async move {
            match signal_listener().await {
                Ok(()) => {
                    warn!("==========================================================================================");
                    warn!("⚠️  Attention - Starting the graceful shutdown procedure (ETA: 30 seconds)...");
                    warn!("⚠️  Attention - To avoid DATA CORRUPTION, do NOT interrupt snarkOS (or press Ctrl+C again)");
                    warn!("⚠️  Attention - Please wait until the shutdown gracefully completes (ETA: 30 seconds)");
                    warn!("==========================================================================================");

                    match node_clone.get() {
                        // If the node is already initialized, then shut it down.
                        Some(node) => node.shut_down().await,
                        // Otherwise, if the node is not yet initialized, then set the shutdown flag directly.
                        None => shutdown_flag.store(true, Ordering::Relaxed),
                    }

                    // A best-effort attempt to let any ongoing activity conclude.
                    tokio::time::sleep(Duration::from_secs(3)).await;

                    // Terminate the process.
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
