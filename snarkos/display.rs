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

use crate::config::Config;
use snarkos_network::NodeType;
use snarkvm_dpc::{testnet1::instantiated::Components, Address};

use colored::*;

use std::str::FromStr;
use tracing_subscriber::EnvFilter;

pub fn initialize_logger(config: &Config) {
    match config.node.verbose {
        0 => {}
        verbosity => {
            match verbosity {
                1 => std::env::set_var("RUST_LOG", "info"),
                2 => std::env::set_var("RUST_LOG", "debug"),
                3 | 4 => std::env::set_var("RUST_LOG", "trace"),
                _ => std::env::set_var("RUST_LOG", "info"),
            };

            // disable undesirable logs
            let filter = EnvFilter::from_default_env().add_directive("mio=off".parse().unwrap());

            // initialize tracing
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(config.node.verbose == 4)
                .init();
        }
    }
}

pub fn print_welcome(config: &Config) {
    println!("{}", render_welcome(config));
}

fn render_welcome(config: &Config) -> String {
    let mut output = String::new();

    output += &r#"

         ╦╬╬╬╬╬╦
        ╬╬╬╬╬╬╬╬╬                    ▄▄▄▄        ▄▄▄
       ╬╬╬╬╬╬╬╬╬╬╬                  ▐▓▓▓▓▌       ▓▓▓
      ╬╬╬╬╬╬╬╬╬╬╬╬╬                ▐▓▓▓▓▓▓▌      ▓▓▓     ▄▄▄▄▄▄       ▄▄▄▄▄▄
     ╬╬╬╬╬╬╬╬╬╬╬╬╬╬╬              ▐▓▓▓  ▓▓▓▌     ▓▓▓   ▄▓▓▀▀▀▀▓▓▄   ▐▓▓▓▓▓▓▓▓▌
    ╬╬╬╬╬╬╬╜ ╙╬╬╬╬╬╬╬            ▐▓▓▓▌  ▐▓▓▓▌    ▓▓▓  ▐▓▓▓▄▄▄▄▓▓▓▌ ▐▓▓▓    ▓▓▓▌
   ╬╬╬╬╬╬╣     ╠╬╬╬╬╬╬           █▓▓▓▓▓▓▓▓▓▓█    ▓▓▓  ▐▓▓▀▀▀▀▀▀▀▀▘ ▐▓▓▓    ▓▓▓▌
  ╬╬╬╬╬╬╣       ╠╬╬╬╬╬╬         █▓▓▓▌    ▐▓▓▓█   ▓▓▓   ▀▓▓▄▄▄▄▓▓▀   ▐▓▓▓▓▓▓▓▓▌
 ╬╬╬╬╬╬╣         ╠╬╬╬╬╬╬       ▝▀▀▀▀      ▀▀▀▀▘  ▀▀▀     ▀▀▀▀▀▀       ▀▀▀▀▀▀
╚╬╬╬╬╬╩           ╩╬╬╬╬╩

"#
    .white()
    .bold();

    output += &"Welcome to Aleo! We thank you for running a network node and supporting privacy.\n\n".bold();

    let mut is_miner = config.miner.is_miner;
    if is_miner {
        match Address::<Components>::from_str(&config.miner.miner_address) {
            Ok(miner_address) => {
                output += &format!("Your Aleo address is {}.\n\n", miner_address)
                    .bold()
                    .to_string();
            }
            Err(_) => {
                output +=
                    &"Miner not started. Please specify a valid miner address in your ~/.snarkOS/config.toml file or by using the --miner-address option in the CLI.\n\n"
                .red().bold();

                is_miner = false;
            }
        }
    }

    let network = match config.aleo.network_id {
        0 => "mainnet".to_string(),
        i => format!("testnet{}", i),
    };

    match config.node.kind {
        NodeType::Client if is_miner => {
            output += &format!("Starting a mining node on {}.\n", network).bold().to_string();
        }
        NodeType::Client => {
            output += &format!("Starting a client node on {}.\n", network).bold().to_string();
        }
        NodeType::Crawler => output += &format!("Starting a crawler node on {}.\n", network).bold().to_string(),
        NodeType::Beacon => output += &format!("Starting a beacon node on {}.\n", network).bold().to_string(),
        NodeType::SyncProvider => {
            output += &format!("Starting a sync provider node on {}.\n", network)
                .bold()
                .to_string()
        }
    }

    output
}
