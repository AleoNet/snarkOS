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

use snarkos_node_narwhal::{helpers::init_primary_channels, Primary, Shared};

use anyhow::{bail, Result};
use std::{str::FromStr, sync::Arc};
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
};

type CurrentNetwork = snarkvm::prelude::Testnet3;

/// Initializes the logger.
pub fn initialize_logger(verbosity: u8) {
    match verbosity {
        0 => std::env::set_var("RUST_LOG", "info"),
        1 => std::env::set_var("RUST_LOG", "debug"),
        2 | 3 | 4 => std::env::set_var("RUST_LOG", "trace"),
        _ => std::env::set_var("RUST_LOG", "info"),
    };

    // Filter out undesirable logs. (unfortunately EnvFilter cannot be cloned)
    let [filter] = std::array::from_fn(|_| {
        let filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper=off".parse().unwrap())
            .add_directive("reqwest=off".parse().unwrap())
            .add_directive("want=off".parse().unwrap())
            .add_directive("warp=off".parse().unwrap());

        if verbosity > 3 {
            filter.add_directive("snarkos_node_tcp=trace".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_tcp=off".parse().unwrap())
        }
    });

    // Initialize tracing.
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::Layer::default().with_target(verbosity > 2).with_filter(filter))
        .try_init();
}

/// Starts the primary instance.
pub async fn start_primary(node_id: u16) -> Result<Primary<CurrentNetwork>> {
    // Initialize the shared state.
    let shared = Arc::new(Shared::<CurrentNetwork>::new());
    // Initialize the primary channels.
    let (sender, receiver) = init_primary_channels();
    // Initialize the primary instance.
    let mut primary = Primary::<CurrentNetwork>::new(shared.clone(), Some(node_id))?;
    // Run the primary instance.
    primary.run(receiver).await?;
    // Return the primary instance.
    Ok(primary)
}

#[tokio::main]
async fn main() -> Result<()> {
    initialize_logger(3);

    // Retrieve the command-line arguments.
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        bail!("Please provide a command.")
    }

    // Parse the node ID.
    let node_id = u16::from_str(&args[1])?;

    // Start the primary instance.
    let _ = start_primary(node_id).await?;

    println!("Hello, world!");

    Ok(())
}
