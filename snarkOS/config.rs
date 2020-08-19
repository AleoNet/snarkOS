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

use crate::{
    cli::CLI,
    parameters::{flag, option, types::*},
};
use snarkos_errors::node::CliError;

use clap::ArgMatches;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use toml;

/// Hardcoded bootnodes maintained by Aleo.
/// A node should try and connect to these first after coming online.
pub const MAINNET_BOOTNODES: &'static [&str] = &[]; // "192.168.0.1:4130"
pub const TESTNET_BOOTNODES: &'static [&str] = &["50.18.83.123:4131"]; // "192.168.0.1:4131"

/// Represents all configuration options for a node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub aleo: Aleo,
    pub node: Node,
    pub miner: Miner,
    pub rpc: JsonRPC,
    pub p2p: P2P,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aleo {
    pub network_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRPC {
    pub json_rpc: bool,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub dir: PathBuf,
    pub db: String,
    pub is_bootnode: bool,
    pub ip: String,
    pub port: u16,
    pub quiet: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Miner {
    pub is_miner: bool,
    pub miner_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct P2P {
    pub bootnodes: Vec<String>,
    pub mempool_interval: u8,
    pub min_peers: u16,
    pub max_peers: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aleo: Aleo { network_id: 1 },
            node: Node {
                dir: Self::snarkos_dir(),
                db: "snarkos_testnet1".into(),
                is_bootnode: false,
                ip: "0.0.0.0".into(),
                port: 4131,
                quiet: false,
            },
            miner: Miner {
                is_miner: false,
                miner_address: "".into(),
            },
            rpc: JsonRPC {
                json_rpc: true,
                port: 3030,
                // TODO (raychu86) Establish a random username and password for the node operator by default
                username: Some("Username".into()),
                password: Some("Password".into()),
            },
            p2p: P2P {
                bootnodes: TESTNET_BOOTNODES
                    .iter()
                    .map(|node| (*node).to_string())
                    .collect::<Vec<String>>(),
                mempool_interval: 5,
                min_peers: 2,
                max_peers: 20,
            },
        }
    }
}

impl Config {
    /// The directory that snarkOS system files will be stored
    fn snarkos_dir() -> PathBuf {
        let mut path = home_dir().unwrap_or(std::env::current_dir().unwrap());
        path.push(".snarkOS/");

        path
    }

    /// Read the config from the snarkOS.toml file
    fn read_config() -> Result<Self, CliError> {
        let snarkos_path = Self::snarkos_dir();
        let mut config_path = snarkos_path.clone();
        config_path.push("snarkOS.toml");

        if !Path::exists(&config_path) {
            // Create a new default snarkOS.toml file if it doesn't already exist
            fs::create_dir_all(&snarkos_path)?;

            let default_config_string = toml::to_string(&Config::default())?;

            fs::write(&config_path, default_config_string)?;
        }

        let toml_string = match fs::read_to_string(&config_path) {
            Ok(toml) => toml,
            Err(_) => {
                fs::create_dir_all(&snarkos_path)?;
                String::new()
            }
        };

        // Parse the contents into the `Config` struct
        let config: Config = toml::from_str(&toml_string)?;

        Ok(config)
    }

    fn parse(&mut self, arguments: &ArgMatches, options: &[&str]) {
        options.iter().for_each(|option| match *option {
            // Flags
            "is-bootnode" => self.is_bootnode(arguments.is_present(option)),
            "is-miner" => self.is_miner(arguments.is_present(option)),
            "no-jsonrpc" => self.no_jsonrpc(arguments.is_present(option)),
            "quiet" => self.quiet(arguments.is_present(option)),
            // Options
            "connect" => self.connect(arguments.value_of(option)),
            "ip" => self.ip(arguments.value_of(option)),
            "miner-address" => self.miner_address(arguments.value_of(option)),
            "mempool-interval" => self.mempool_interval(clap::value_t!(arguments.value_of(*option), u8).ok()),
            "max-peers" => self.max_peers(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "min-peers" => self.min_peers(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "network" => self.network(clap::value_t!(arguments.value_of(*option), u8).ok()),
            "path" => self.path(arguments.value_of(option)),
            "port" => self.port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "rpc-port" => self.rpc_port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "rpc-username" => self.rpc_username(arguments.value_of(option)),
            "rpc-password" => self.rpc_password(arguments.value_of(option)),
            _ => (),
        });
    }

    /// Sets `network` to the specified network, overriding its previous state.
    fn network(&mut self, argument: Option<u8>) {
        if let Some(network_id) = argument {
            match network_id {
                0 => {
                    self.node.db = "snarkos_mainnet".into();
                    self.node.port = 4130;
                    self.p2p.bootnodes = MAINNET_BOOTNODES
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();
                    self.aleo.network_id = network_id;
                }
                _ => {
                    self.node.db = format!("snarkos_testnet{}", network_id);
                    self.node.port = 4130 + (network_id as u16);
                    self.p2p.bootnodes = TESTNET_BOOTNODES
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();
                    self.aleo.network_id = network_id;
                }
            }
        }
    }

    fn no_jsonrpc(&mut self, argument: bool) {
        self.rpc.json_rpc = !argument;
    }

    fn is_bootnode(&mut self, argument: bool) {
        self.node.is_bootnode = argument;
        if argument {
            self.p2p.bootnodes = vec![];
        }
    }

    fn is_miner(&mut self, argument: bool) {
        self.miner.is_miner = argument;
    }

    fn quiet(&mut self, argument: bool) {
        self.node.quiet = argument;
    }

    fn ip(&mut self, argument: Option<&str>) {
        if let Some(ip) = argument {
            self.node.ip = ip.to_string();
        }
    }

    fn port(&mut self, argument: Option<u16>) {
        if let Some(port) = argument {
            self.node.port = port;
        }
    }

    fn path(&mut self, argument: Option<&str>) {
        match argument {
            Some(path) => self.node.db = path.into(),
            _ => (),
        };
    }

    fn connect(&mut self, argument: Option<&str>) {
        if let Some(bootnode) = argument {
            self.p2p.bootnodes = vec![bootnode.to_string()];
        }
    }

    fn miner_address(&mut self, argument: Option<&str>) {
        if let Some(miner_address) = argument {
            self.miner.miner_address = miner_address.to_string();
        }
    }

    fn mempool_interval(&mut self, argument: Option<u8>) {
        if let Some(interval) = argument {
            self.p2p.mempool_interval = interval
        }
    }

    fn min_peers(&mut self, argument: Option<u16>) {
        if let Some(num_peers) = argument {
            self.p2p.min_peers = num_peers;
        }
    }

    fn max_peers(&mut self, argument: Option<u16>) {
        if let Some(num_peers) = argument {
            self.p2p.max_peers = num_peers;
        }
    }

    fn rpc_port(&mut self, argument: Option<u16>) {
        if let Some(rpc_port) = argument {
            self.rpc.port = rpc_port;
        }
    }

    fn rpc_username(&mut self, argument: Option<&str>) {
        if let Some(username) = argument {
            self.rpc.username = Some(username.to_string());
        }
    }

    fn rpc_password(&mut self, argument: Option<&str>) {
        if let Some(password) = argument {
            self.rpc.password = Some(password.to_string());
        }
    }
}

/// Parses command line arguments into node configuration parameters.
pub struct ConfigCli;

impl CLI for ConfigCli {
    type Config = Config;

    const ABOUT: AboutType = "Run an Aleo node (include -h for more options)";
    const FLAGS: &'static [FlagType] = &[flag::NO_JSONRPC, flag::IS_BOOTNODE, flag::IS_MINER, flag::QUIET];
    const NAME: NameType = "snarkOS";
    const OPTIONS: &'static [OptionType] = &[
        option::IP,
        option::PORT,
        option::PATH,
        option::CONNECT,
        option::MINER_ADDRESS,
        option::MEMPOOL_INTERVAL,
        option::MIN_PEERS,
        option::MAX_PEERS,
        option::NETWORK,
        option::RPC_PORT,
        option::RPC_USERNAME,
        option::RPC_PASSWORD,
    ];
    const SUBCOMMANDS: &'static [SubCommandType] = &[];

    /// Handle all CLI arguments and flags for skeleton node
    fn parse(arguments: &ArgMatches) -> Result<Self::Config, CliError> {
        let mut config = Config::read_config()?;
        config.parse(arguments, &[
            "network",
            "no-jsonrpc",
            "is-bootnode",
            "is-miner",
            "quiet",
            "ip",
            "port",
            "path",
            "connect",
            "miner-address",
            "mempool-interval",
            "min-peers",
            "max-peers",
            "rpc-port",
            "rpc-username",
            "rpc-password",
        ]);

        Ok(config)
    }
}
