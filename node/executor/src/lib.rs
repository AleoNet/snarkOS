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

#![forbid(unsafe_code)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate tracing;

mod macros;

mod node_type;
pub use node_type::*;

mod resources;
pub use resources::Resource;

mod status;
pub use status::*;

use crate::resources::Resources;

use once_cell::sync::OnceCell;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::Arc;

#[async_trait]
pub trait Executor: 'static + Clone + Send + Sync {
    /// The node type.
    const NODE_TYPE: NodeType;

    /// Returns the node type.
    fn node_type() -> NodeType {
        Self::NODE_TYPE
    }

    /// Returns the status of the node.
    fn status() -> &'static RawStatus {
        static STATUS: OnceCell<RawStatus> = OnceCell::new();
        STATUS.get_or_init(RawStatus::default)
    }

    /// Returns the resource handler for the node.
    fn resources() -> &'static Resources {
        static RESOURCES: OnceCell<Resources> = OnceCell::new();
        RESOURCES.get_or_init(Resources::default)
    }

    /// Returns a thread pool for the executor to perform intensive operations.
    fn executor_pool() -> &'static Arc<ThreadPool> {
        static POOL: OnceCell<Arc<ThreadPool>> = OnceCell::new();
        POOL.get_or_init(|| {
            Arc::new(
                ThreadPoolBuilder::new()
                    .stack_size(8 * 1024 * 1024)
                    .num_threads((num_cpus::get() * 7 / 8).max(2))
                    .build()
                    .expect("Failed to initialize a thread pool for the node"),
            )
        })
    }

    /// Handles OS signals for the node to intercept and perform a clean shutdown.
    /// Note: Only Ctrl-C is supported; it should work on both Unix-family systems and Windows.
    fn handle_signals(&self) {
        let node = self.clone();
        Self::resources().register_task(
            None, // No need to provide an id, as the task will run indefinitely.
            tokio::task::spawn(async move {
                match tokio::signal::ctrl_c().await {
                    Ok(()) => {
                        node.shut_down().await;
                        std::process::exit(0);
                    }
                    Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
                }
            }),
        );
    }

    /// Disconnects from peers and shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");
        // Update the node status.
        Self::status().update(Status::ShuttingDown);

        // Flush the tasks.
        Self::resources().shut_down();
        trace!("Node has shut down.");
    }
}
