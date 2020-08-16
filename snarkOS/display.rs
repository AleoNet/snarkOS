use crate::config::Config;
use snarkos_dpc::base_dpc::instantiated::Components;
use snarkos_objects::AccountAddress;

use colored::*;
use std::str::FromStr;

pub fn render_init(config: &Config) -> String {
    let mut output = String::new();

    output += &format!(
        r#"

         ╦╬╬╬╬╦
        ╬╬╬╬╬╬╠╬                 ▄▄▄▄▄▄     ▄▄▄▄       ▄▄▄▄▄▄▄▄▄▄   ▄▓▓▓▓▓▓▓▄
       ╬╬╬╬╬╬╬╬╬╬               ▓▓▓▓▓▓▓▓▌  ▐▓▓▓▓      ▐▓▓▓▓▓▓▓▓▓▌ ▄▓▓▓▓▓▓▓▓▓▓▓
      ╬╬╬╬╬╬╬╬╬╬╬╬             ▓▓▓▓  ▓▓▓█  ▐▓▓▓▓      ▐▓▓▓▓       ▓▓▓▓▌   ▓▓▓▓▌
     ╬╬╬╬╬╬╜╙╬╬╬╬╬╬           ▐▓▓▓▓  ▓▓▓▓▌ ▐▓▓▓▓      ▐▓▓▓▓▓▓▓▓▓  ▓▓▓▓▌   ▓▓▓▓▌
    ╬╬╬╬╬╬    ╬╬╬╬╬╬          ▓▓▓▓▓▓▓▓▓▓▓█ ▐▓▓▓▓      ▐▓▓▓▌       ▓▓▓▓▌   ▓▓▓▓▌
   ╬╬╬╬╬╬      ╬╬╬╬╬╬         ▓▓▓▓    ▓▓▓▓ ▐▓▓▓▓▓▓▓▓▓ ▐▓▓▓▓▓▓▓▓▓▌ ▀▓▓▓▓▓▓▓▓▓▓▓
  ╬╬╬╬╬╬        ╬╬╬╬╬╬        ▀▀▀▀    ▀▀▀▀  ▀▀▀▀▀▀▀▀▀  ▀▀▀▀▀▀▀▀▀▀   ▀▀█████▀▀
   ╙╙╙╙          ╙╙╙╙

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
                output += &format!("Your Aleo address is {}.\n\n", config.miner.miner_address)
                    .bold()
                    .to_string();
            }
            Err(_) => output += &format!(
                "Miner not started. Please specify a valid miner address in your ~/.snarkOS/snarkOS.toml file or by using the --miner-address option in the CLI."
            ).bold()
                .to_string()
        }
    }

    let network = match config.aleo.network_id {
        0 => "mainnet".to_string(),
        i => format!("testnet{}", i),
    };
    if config.miner.is_miner {
        output += &format!("Starting a full node on {}.\n\n", network).bold().to_string();
    } else {
        output += &format!("Starting a light client node on {}.\n\n", network)
            .bold()
            .to_string();
    }

    if config.rpc.json_rpc {
        output += &format!("Listening for RPC requests on port {}\n", config.rpc.port);
    }

    format!("{}", output)
}
