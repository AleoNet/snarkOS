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

use snarkos::Node;

use anyhow::Result;
use structopt::StructOpt;
use tokio::runtime;

fn main() -> Result<()> {
    // Initialize the runtime configuration.
    let runtime = runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .worker_threads(num_cpus::get().saturating_sub(1).max(1)) // Don't use 100% of the cores
        .max_blocking_threads(num_cpus::get().saturating_sub(1).max(1)) // Don't use 100% of the cores
        .build()?;

    // Initialize the parallelization parameters.
    rayon::ThreadPoolBuilder::new()
        .stack_size(8 * 1024 * 1024)
        .num_threads((num_cpus::get() / 4 * 3).max(1))
        .build_global()
        .unwrap();

    runtime.block_on(Node::from_args().start())?;

    Ok(())
}
