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

use snarkvm::prelude::*;

use colored::*;
use std::fmt::Write;

pub fn initialize_logger(verbosity: u8) {
    match verbosity {
        0 => env::set_var("RUST_LOG", "info"),
        1 => env::set_var("RUST_LOG", "debug"),
        2 | 3 => env::set_var("RUST_LOG", "trace"),
        _ => env::set_var("RUST_LOG", "info"),
    };

    // Filter out undesirable logs.
    let filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("hyper::client=off".parse().unwrap())
        .add_directive("hyper::proto=off".parse().unwrap())
        .add_directive("jsonrpsee=off".parse().unwrap())
        .add_directive("mio=off".parse().unwrap())
        .add_directive("rusoto_core=off".parse().unwrap())
        .add_directive("tokio_util=off".parse().unwrap())
        .add_directive("want=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap());

    // Initialize tracing.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(verbosity == 3)
        .try_init();
}

pub fn welcome_message() -> String {
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
    output += &"Welcome to Aleo! We thank you for running a node and supporting privacy.\n".bold();
    output
}

pub fn notification_message<N: Network>(miner: Option<Address<N>>) -> String {
    let mut output = String::new();
    output += &r#"

 ==================================================================================================

                         Welcome to Aleo Testnet3 - Incentivization Period

 ==================================================================================================

     The incentivized testnet will be announced on Discord. Visit https://www.aleo.org/discord
     for the official launch announcement on Month Date, Year, from the Aleo team.

     Minimum requirements:

         - CPU: 16-cores (32-cores preferred)
         - RAM: 16GB of memory (32GB preferred)
         - Storage: 128GB of disk space
         - Network: 50 Mbps of upload and download bandwidth

     To get started (after Month Date, Year):

         1. Generate one Aleo account, and save the account private key and view key.
         2. Ensure ports 4132/tcp and 3032/tcp are open on your router and OS firewall.
         3. Ensure your Aleo node is running the `run-client.sh` or `run-miner.sh` script,
            in order to automatically stay up to date on the incentivized testnet.
         4. File technical issues on Github at https://github.com/AleoHQ/snarkOS/issues/new/choose
         5. Ask questions on Discord at https://www.aleo.org/discord
         6. Please be respectful to all members of the Aleo community.

     To claim rewards (after Month Date, Year):

         1. Participants will be required to KYC at the end of incentivized testnet3.
         2. Participants must demonstrate ownership of their Aleo miner address.
         3. [For United States & Canada] Participants must be accredited investors.
         4. The Aleo team reserves the right to maintain discretion in rewarding participants.

     Thank you for participating in incentivized testnet3 and for supporting privacy!

 ==================================================================================================
"#
    .white()
    .bold();

    if let Some(miner) = miner {
        let _ = write!(
            output,
            "
     Your Aleo miner address is {}

 ==================================================================================================
",
            miner
        );
    }

    output
}
