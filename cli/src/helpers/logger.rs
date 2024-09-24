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

use crate::helpers::{DynamicFormatter, LogWriter};

use crossterm::tty::IsTty;
use std::{
    fs::File,
    io,
    path::Path,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::mpsc;
use tracing_subscriber::{
    layer::{Layer, SubscriberExt},
    util::SubscriberInitExt,
    EnvFilter,
};

/// Initializes the logger.
///
/// ```ignore
/// 0 => info
/// 1 => info, debug
/// 2 => info, debug, trace, snarkos_node_sync=trace
/// 3 => info, debug, trace, snarkos_node_bft=trace
/// 4 => info, debug, trace, snarkos_node_bft::gateway=trace
/// 5 => info, debug, trace, snarkos_node_router=trace
/// 6 => info, debug, trace, snarkos_node_tcp=trace
/// ```
pub fn initialize_logger<P: AsRef<Path>>(
    verbosity: u8,
    nodisplay: bool,
    logfile: P,
    shutdown: Arc<AtomicBool>,
) -> mpsc::Receiver<Vec<u8>> {
    match verbosity {
        0 => std::env::set_var("RUST_LOG", "info"),
        1 => std::env::set_var("RUST_LOG", "debug"),
        2.. => std::env::set_var("RUST_LOG", "trace"),
    };

    // Filter out undesirable logs. (unfortunately EnvFilter cannot be cloned)
    let [filter, filter2] = std::array::from_fn(|_| {
        let filter = EnvFilter::from_default_env()
            .add_directive("mio=off".parse().unwrap())
            .add_directive("tokio_util=off".parse().unwrap())
            .add_directive("hyper=off".parse().unwrap())
            .add_directive("reqwest=off".parse().unwrap())
            .add_directive("want=off".parse().unwrap())
            .add_directive("warp=off".parse().unwrap());

        let filter = if verbosity >= 2 {
            filter.add_directive("snarkos_node_sync=trace".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_sync=debug".parse().unwrap())
        };

        let filter = if verbosity >= 3 {
            filter
                .add_directive("snarkos_node_bft=trace".parse().unwrap())
                .add_directive("snarkos_node_bft::gateway=debug".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_bft=debug".parse().unwrap())
        };

        let filter = if verbosity >= 4 {
            filter.add_directive("snarkos_node_bft::gateway=trace".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_bft::gateway=debug".parse().unwrap())
        };

        let filter = if verbosity >= 5 {
            filter.add_directive("snarkos_node_router=trace".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_router=debug".parse().unwrap())
        };

        if verbosity >= 6 {
            filter.add_directive("snarkos_node_tcp=trace".parse().unwrap())
        } else {
            filter.add_directive("snarkos_node_tcp=off".parse().unwrap())
        }
    });

    // Create the directories tree for a logfile if it doesn't exist.
    let logfile_dir = logfile.as_ref().parent().expect("Root directory passed as a logfile");
    if !logfile_dir.exists() {
        std::fs::create_dir_all(logfile_dir)
            .expect("Failed to create a directories: '{logfile_dir}', please check if user has permissions");
    }
    // Create a file to write logs to.
    let logfile =
        File::options().append(true).create(true).open(logfile).expect("Failed to open the file for writing logs");

    // Initialize the log channel.
    let (log_sender, log_receiver) = mpsc::channel(1024);

    // Initialize the log sender.
    let log_sender = match nodisplay {
        true => None,
        false => Some(log_sender),
    };

    // Initialize tracing.
    let _ = tracing_subscriber::registry()
        .with(
            // Add layer using LogWriter for stdout / terminal
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(log_sender.is_none() && io::stdout().is_tty())
                .with_writer(move || LogWriter::new(&log_sender))
                .with_target(verbosity > 2)
                .event_format(DynamicFormatter::new(shutdown))
                .with_filter(filter),
        )
        .with(
            // Add layer redirecting logs to the file
            tracing_subscriber::fmt::Layer::default()
                .with_ansi(false)
                .with_writer(logfile)
                .with_target(verbosity > 2)
                .with_filter(filter2),
        )
        .try_init();

    log_receiver
}

/// Returns the welcome message as a string.
pub fn welcome_message() -> String {
    use colored::Colorize;

    let mut output = String::new();
    output += &r#"

         ╦╬╬╬╬╬╦
        ╬╬╬╬╬╬╬╬╬                    ▄▄▄▄        ▄▄▄
       ╬╬╬╬╬╬╬╬╬╬╬                  ▐▓▓▓▓▌       ▓▓▓
      ╬╬╬╬╬╬╬╬╬╬╬╬╬                ▐▓▓▓▓▓▓▌      ▓▓▓     ▄▄▄▄▄▄       ▄▄▄▄▄▄
     ╬╬╬╬╬╬╬╬╬╬╬╬╬╬╬              ▐▓▓▓  ▓▓▓▌     ▓▓▓   ▄▓▓▀▀▀▀▓▓▄   ▐▓▓▓▓▓▓▓▓▌
    ╬╬╬╬╬╬╬╜ ╙╬╬╬╬╬╬╬            ▐▓▓▓▌  ▐▓▓▓▌    ▓▓▓  ▐▓▓▓▄▄▄▄▓▓▓▌ ▐▓▓▓    ▓▓▓▌
   ╬╬╬╬╬╬╣     ╠╬╬╬╬╬╬           ▓▓▓▓▓▓▓▓▓▓▓▓    ▓▓▓  ▐▓▓▀▀▀▀▀▀▀▀▘ ▐▓▓▓    ▓▓▓▌
  ╬╬╬╬╬╬╣       ╠╬╬╬╬╬╬         ▓▓▓▓▌    ▐▓▓▓▓   ▓▓▓   ▀▓▓▄▄▄▄▓▓▀   ▐▓▓▓▓▓▓▓▓▌
 ╬╬╬╬╬╬╣         ╠╬╬╬╬╬╬       ▝▀▀▀▀      ▀▀▀▀▘  ▀▀▀     ▀▀▀▀▀▀       ▀▀▀▀▀▀
╚╬╬╬╬╬╩           ╩╬╬╬╬╩


"#
    .white()
    .bold();
    output += &"👋 Welcome to Aleo! We thank you for running a node and supporting privacy.\n".bold();
    output
}
