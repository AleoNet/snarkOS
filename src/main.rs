// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use snarkos::{Miner, Node};

use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};

use ::rand::thread_rng;
use anyhow::Result;
use tracing_subscriber::EnvFilter;

pub fn initialize_logger() {
    let verbosity = 4;

    match verbosity {
        1 => std::env::set_var("RUST_LOG", "info"),
        2 => std::env::set_var("RUST_LOG", "debug"),
        3 | 4 => std::env::set_var("RUST_LOG", "trace"),
        _ => std::env::set_var("RUST_LOG", "info"),
    };

    // Filter out undesirable logs.
    let filter = EnvFilter::from_default_env()
        .add_directive("mio=off".parse().unwrap())
        .add_directive("tokio_util=off".parse().unwrap());

    // Initialize tracing.
    tracing_subscriber::fmt().with_env_filter(filter).with_target(verbosity == 4).init();
}

#[tokio::main]
async fn main() -> Result<()> {
    let port = env::args().nth(1).unwrap_or_else(|| "4132".to_string()).parse()?;

    initialize_logger();
    tracing::trace!("Hello world");

    let account = Account::<Testnet2>::new(&mut thread_rng());

    let node = Node::<Testnet2, Miner>::new()?;
    node.start_listener(port).await?;
    node.connect_to("127.0.0.1:4133".parse().unwrap()).await;
    if port == 4134 {
        node.start_miner(account.address())?;
    }

    std::future::pending::<()>().await;
    Ok(())
}
