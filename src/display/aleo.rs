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

use snarkvm::dpc::prelude::*;

use colored::*;

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
    output += &"Welcome to Aleo! We thank you for running a network node and supporting privacy.\n".bold();
    output
}

pub fn notification_message<N: Network>(miner: Option<Address<N>>) -> String {
    let mut output = String::new();
    output += &r#"

 ==================================================================================================

                         Welcome to Aleo Testnet2 - Incentivization Period

 ==================================================================================================

     The incentivized testnet will be announced on Discord. Visit https://discord.gg/5v2ynrw2ds
     for the official launch announcement on November 30, 2021, from the Aleo team.

     Minimum requirements:

         - CPU: 16-cores (32-cores preferred)
         - RAM: 16GB of memory (32GB preferred)
         - Storage: 128GB of disk space
         - Network: 50 Mbps of upload and download bandwidth

     To get started (after November 30, 2021):

         1. Generate one Aleo account, and save the account private key and view key.
         2. Ensure ports 4132/tcp and 3032/tcp are open on your router and OS firewall.
         3. Ensure your Aleo node is running the `run-client.sh` or `run-miner.sh` script,
            in order to automatically stay up to date on the incentivized testnet.
         4. File technical issues on Github at https://github.com/AleoHQ/snarkOS/issues/new/choose
         5. Ask questions on Discord at https://discord.gg/5v2ynrw2ds
         6. Please be respectful to all members of the Aleo community.

     To claim rewards (after February 15, 2022):

         1. Participants will be required to KYC at the end of incentivized testnet2.
         2. Participants must demonstrate ownership of their Aleo miner address.
         3. [For United States & Canada] Participants must be accredited investors.
         4. The Aleo team reserves the right to maintain discretion in rewarding participants.

     Thank you for participating in incentivized testnet2 and for supporting privacy!

 ==================================================================================================
"#
    .white()
    .bold();

    if let Some(miner) = miner {
        output += &format!(
            "
     Your Aleo miner address is {}

 ==================================================================================================
",
            miner
        );
    }

    output
}
