// Copyright (C) 2019-2020 Aleo Systems Inc.
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
use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_objects::AccountAddress;

use colored::*;
use std::str::FromStr;

pub fn render_init(config: &Config) -> String {
    let mut output = String::new();

    output += &format!(
        r#"

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
    )
    .white()
    .bold()
    .to_string();

    output += &format!("Welcome to Aleo! We thank you for running a network node and supporting privacy.\n\n")
        .bold()
        .to_string();

    if config.miner.is_miner {
        match AccountAddress::<Components>::from_str(&config.miner.miner_address) {
            Ok(miner_address) => {
                output += &format!("Your Aleo address is {}.\n\n", miner_address)
                    .bold()
                    .to_string();
            }
            Err(_) => output += &format!(
                "Miner not started. Please specify a valid miner address in your ~/.snarkOS/config.toml file or by using the --miner-address option in the CLI."
            ).bold()
                .to_string()
        }
    }

    let network = match config.aleo.network_id {
        0 => "mainnet".to_string(),
        i => format!("testnet{}", i),
    };
    if config.miner.is_miner {
        output += &format!("Starting a mining node on {}.\n\n", network).bold().to_string();
    } else {
        output += &format!("Starting a client node on {}.\n\n", network).bold().to_string();
    }

    if config.rpc.json_rpc {
        output += &format!("Listening for RPC requests on port {}\n", config.rpc.port);
    }

    format!("{}", output)
}
