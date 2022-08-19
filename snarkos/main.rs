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

use snarkos::{logger::initialize_logger, CLI};

use anyhow::Result;
use clap::Parser;
use tokio::runtime;

fn main() -> Result<()> {
    if num_cpus::get() < 16 {
        eprintln!("\nWARNING - Your machine must have at least 16-cores to run a node.\n");
    }

    // Parse the provided arguments.
    let cli = CLI::parse();

    // Start logging.
    initialize_logger(cli.verbosity);

    let (num_tokio_worker_threads, max_tokio_blocking_threads) = if !cli.beacon {
        ((num_cpus::get() / 8 * 2).max(1), num_cpus::get())
    } else {
        (num_cpus::get(), 512) // 512 is tokio's current default
    };

    // Initialize the runtime configuration.
    let runtime = runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .worker_threads(num_tokio_worker_threads)
        .max_blocking_threads(max_tokio_blocking_threads)
        .build()?;

    let num_rayon_cores_global = if !cli.beacon {
        (num_cpus::get() / 8 * 5).max(1)
    } else {
        num_cpus::get()
    };

    // Initialize the parallelization parameters.
    rayon::ThreadPoolBuilder::new()
        .stack_size(8 * 1024 * 1024)
        .num_threads(num_rayon_cores_global)
        .build_global()
        .unwrap();

    runtime.block_on(async move {
        cli.start().await.expect("Failed to start the node");
    });

    Ok(())
}
