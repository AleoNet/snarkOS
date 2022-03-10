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

use clap::Parser;
#[cfg(feature = "postgres")]
use snarkos_crawler::storage::initialize_storage;
use snarkos_crawler::{
    constants::SYNC_NODES,
    crawler::{Crawler, Opts},
};

#[tokio::main]
async fn main() {
    // Enable tracing.
    snarkos_synthetic_node::enable_tracing();

    // Read configuration options.
    let opts = Opts::parse();

    // Initialize the storage, if enabled.
    #[cfg(feature = "postgres")]
    let storage = Some(initialize_storage(&opts).await.expect("couldn't initialize storage"));
    #[cfg(not(feature = "postgres"))]
    let storage = None;

    // Configure and start crawler.
    let crawler = Crawler::new(opts, storage).await;

    // Register the addresses of the sync nodes.
    for addr in SYNC_NODES {
        let addr = addr.parse().unwrap();
        crawler.known_network.add_node(addr);
    }

    // Start crawling.
    crawler.run_periodic_tasks();

    std::future::pending::<()>().await;
}
