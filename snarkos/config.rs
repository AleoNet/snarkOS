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

use crate::{
    cli::CLI,
    errors::CliError,
    parameters::{flag, option, subcommand, types::*},
    update::UpdateCLI,
};

use snarkos_network::NodeType;

use clap::ArgMatches;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

/// Peer discovery nodes maintained by Aleo.
/// A node should try and connect to these first after coming online.
pub const MAINNET_BEACONS: &[&str] = &[]; // "192.168.0.1:4130"
pub const TESTNET_BEACONS: &[&str] = &[
    "164.90.244.192:4131",
    "165.227.251.158:4131",
    "159.203.49.11:4131",
    "67.207.71.112:4131",
    "138.68.122.60:4131",
    "164.90.241.245:4131",
    "139.59.48.164:4131",
    "188.166.196.149:4131",
    "34.84.79.188:4131",
    "54.153.131.7:4131",
];

pub const MAINNET_SYNC_PROVIDERS: &[&str] = &[];
pub const TESTNET_SYNC_PROVIDERS: &[&str] = &[
    "178.128.128.92:4131",
    "143.244.213.137:4131",
    "159.203.55.128:4131",
    "159.65.211.80:4131",
    "64.225.80.138:4131",
    "138.68.127.67:4131",
    "139.59.52.218:4131",
    "188.166.205.27:4131",
    "34.64.246.68:4131",
    "54.253.223.255:4131",
];

/// Represents all configuration options for a node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub aleo: Aleo,
    pub node: Node,
    pub miner: Miner,
    pub rpc: JsonRPC,
    pub p2p: P2P,
    pub storage: Storage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aleo {
    pub network_id: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRPC {
    pub json_rpc: bool,
    pub ip: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub kind: NodeType,
    pub dir: PathBuf,
    pub db: String,
    pub ip: String,
    pub port: u16,
    pub verbose: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Miner {
    pub is_miner: bool,
    pub miner_address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct P2P {
    #[serde(skip_serializing, skip_deserializing)]
    pub beacons: Vec<String>,
    #[serde(skip_serializing, skip_deserializing)]
    pub sync_providers: Vec<String>,
    #[serde(alias = "mempool_interval")]
    pub mempool_sync_interval: u8,
    pub block_sync_interval: u16,
    pub peer_sync_interval: u16,
    pub min_peers: u16,
    pub max_peers: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Storage {
    /// If set, the value specifies the limit on the number of blocks to export; `0` means there is no limit.
    pub export: Option<u32>,
    /// If set, contains the path to the file contained canon blocks exported using the `--export-canon-blocks` option.
    pub import: Option<PathBuf>,
    /// If `true`, checks the node's storage for inconsistencies and attempts to fix any encountered issues.
    pub validate: bool,
    /// If `true`, deletes any superfluous (non-canon) items from the node's storage. Note: it can temporarily increase
    /// the size of the database files, but they will become smaller after a while, when the database has run its
    /// automated maintenance.
    pub trim: bool,
    /// If `true`, scans superfluous blocks for valid forks at boot time. Can take a while.
    pub scan_for_forks: bool,
    /// If `Some`, will reset canon to at most this block height.
    pub max_head: Option<u32>,
    pub no_recanonize: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            aleo: Aleo { network_id: 1 },
            node: Node {
                kind: NodeType::Client,
                dir: Self::snarkos_dir(),
                db: "snarkos_testnet1".into(),
                ip: "0.0.0.0".into(),
                port: 4131,
                verbose: 2,
            },
            miner: Miner {
                is_miner: false,
                miner_address: "".into(),
            },
            rpc: JsonRPC {
                json_rpc: true,
                ip: "0.0.0.0".into(),
                port: 3030,
                // TODO (raychu86) Establish a random username and password for the node operator by default
                username: Some("Username".into()),
                password: Some("Password".into()),
            },
            p2p: P2P {
                beacons: TESTNET_BEACONS
                    .iter()
                    .map(|node| (*node).to_string())
                    .collect::<Vec<String>>(),
                sync_providers: TESTNET_SYNC_PROVIDERS
                    .iter()
                    .map(|node| (*node).to_string())
                    .collect::<Vec<String>>(),
                mempool_sync_interval: 12,
                peer_sync_interval: 15,
                block_sync_interval: 4,
                min_peers: 20,
                max_peers: 50,
            },
            storage: Storage {
                export: None,
                import: None,
                trim: false,
                validate: false,
                scan_for_forks: false,
                no_recanonize: false,
                max_head: None,
            },
        }
    }
}

impl Config {
    /// The directory that snarkOS system files will be stored
    fn snarkos_dir() -> PathBuf {
        let mut path = home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
        path.push(".snarkOS/");

        path
    }

    /// Read the config from the `config.toml` file
    fn read_config() -> Result<Self, CliError> {
        let snarkos_path = Self::snarkos_dir();
        let mut config_path = snarkos_path.clone();
        config_path.push("config.toml");

        // TODO (howardwu): Revert to this logic after testnet, when configs stabilize.
        // if !Path::exists(&config_path) {
        //     // Create a new default `config.toml` file if it doesn't already exist
        //     fs::create_dir_all(&snarkos_path)?;
        //
        //     let default_config_string = toml::to_string(&Config::default())?;
        //
        //     fs::write(&config_path, default_config_string)?;
        // }

        // TODO (howardwu): Revisit this.
        // For now, override the config.toml file each time.
        {
            // Create a new default `config.toml` file if it doesn't already exist
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
        let mut config: Config = toml::from_str(&toml_string)?;

        let (beacons, sync_providers) = match config.aleo.network_id {
            0 => (MAINNET_BEACONS, MAINNET_SYNC_PROVIDERS),
            _ => (TESTNET_BEACONS, TESTNET_SYNC_PROVIDERS),
        };

        config.p2p.beacons = beacons.iter().map(|node| (*node).to_string()).collect::<Vec<String>>();
        config.p2p.sync_providers = sync_providers
            .iter()
            .map(|node| (*node).to_string())
            .collect::<Vec<String>>();

        Ok(config)
    }

    fn parse(&mut self, arguments: &ArgMatches, options: &[&str]) {
        options.iter().for_each(|option| match *option {
            // Flags
            "is-miner" => self.is_miner(arguments.is_present(option)),
            "no-jsonrpc" => self.no_jsonrpc(arguments.is_present(option)),
            "trim-storage" => self.trim_storage(arguments.is_present(option)),
            "validate-storage" => self.validate_storage(arguments.is_present(option)),
            "no-recanonize" => self.storage.no_recanonize = arguments.is_present(option),
            // Options
            "connect" => self.connect(arguments.value_of(option)),
            "export-canon-blocks" => self.export_canon_blocks(clap::value_t!(arguments.value_of(*option), u32).ok()),
            "import-canon-blocks" => self.import_canon_blocks(arguments.value_of(option)),
            "ip" => self.ip(arguments.value_of(option)),
            "miner-address" => self.miner_address(arguments.value_of(option)),
            "mempool-interval" => self.mempool_interval(clap::value_t!(arguments.value_of(*option), u8).ok()),
            "max-peers" => self.max_peers(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "min-peers" => self.min_peers(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "network" => self.network(clap::value_t!(arguments.value_of(*option), u8).ok()),
            "path" => self.path(arguments.value_of(option)),
            "port" => self.port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "rpc-ip" => self.rpc_ip(arguments.value_of(option)),
            "rpc-port" => self.rpc_port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "rpc-username" => self.rpc_username(arguments.value_of(option)),
            "rpc-password" => self.rpc_password(arguments.value_of(option)),
            "verbose" => self.verbose(clap::value_t!(arguments.value_of(*option), u8).ok()),
            "max-head" => self.storage.max_head = clap::value_t!(arguments.value_of(*option), u32).ok(),
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
                    self.p2p.beacons = MAINNET_BEACONS
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();

                    self.p2p.sync_providers = MAINNET_SYNC_PROVIDERS
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();

                    self.aleo.network_id = network_id;
                }
                _ => {
                    self.node.db = format!("snarkos_testnet{}", network_id);
                    self.node.port = 4130 + (network_id as u16);
                    self.p2p.beacons = TESTNET_BEACONS
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();

                    self.p2p.sync_providers = TESTNET_SYNC_PROVIDERS
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

    fn import_canon_blocks(&mut self, argument: Option<&str>) {
        if let Some(path) = argument {
            self.storage.import = Some(path.to_owned().into());
        }
    }

    fn is_miner(&mut self, argument: bool) {
        self.miner.is_miner = argument;
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
        if let Some(path) = argument {
            self.node.db = path.into();
        }
    }

    fn connect(&mut self, argument: Option<&str>) {
        if let Some(addrs) = argument {
            let sanitized_addrs = addrs.replace(&['[', ']', ' '][..], "");
            let addrs: Vec<String> = sanitized_addrs.split(',').map(|s| s.to_string()).collect();
            self.p2p.beacons = addrs;
        }
    }

    fn export_canon_blocks(&mut self, argument: Option<u32>) {
        self.storage.export = argument;
    }

    fn miner_address(&mut self, argument: Option<&str>) {
        if let Some(miner_address) = argument {
            self.miner.miner_address = miner_address.to_string();
        }
    }

    fn mempool_interval(&mut self, argument: Option<u8>) {
        if let Some(interval) = argument {
            self.p2p.mempool_sync_interval = interval
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

    fn rpc_ip(&mut self, argument: Option<&str>) {
        if let Some(ip) = argument {
            self.rpc.ip = ip.to_string();
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

    fn trim_storage(&mut self, argument: bool) {
        self.storage.trim = argument;
    }

    fn validate_storage(&mut self, argument: bool) {
        self.storage.validate = argument;
    }

    fn verbose(&mut self, argument: Option<u8>) {
        if let Some(verbose) = argument {
            self.node.verbose = verbose
        }
    }

    pub fn check(&self) -> Result<(), CliError> {
        // Check that the minimum and maximum number of peers is valid.
        if self.p2p.min_peers == 0 || self.p2p.max_peers == 0 || self.p2p.min_peers > self.p2p.max_peers {
            return Err(CliError::PeerCountInvalid);
        }

        // Check that the sync interval is a reasonable number of seconds.
        if !(2..=300).contains(&self.p2p.peer_sync_interval) || !(2..=300).contains(&self.p2p.block_sync_interval) {
            return Err(CliError::SyncIntervalInvalid);
        }

        if self.miner.is_miner {
            match self.node.kind {
                NodeType::Client => {}
                NodeType::Crawler | NodeType::Beacon | NodeType::SyncProvider => return Err(CliError::CantMine),
            }
        }

        // TODO (howardwu): Check the memory pool interval.

        Ok(())
    }
}

/// Parses command line arguments into node configuration parameters.
pub struct ConfigCli;

impl CLI for ConfigCli {
    type Config = Config;

    const ABOUT: AboutType = "Run an Aleo node (include -h for more options)";
    const FLAGS: &'static [FlagType] = &[
        flag::NO_JSONRPC,
        flag::IS_MINER,
        flag::TRIM_STORAGE,
        flag::VALIDATE_STORAGE,
        flag::NO_RECANONIZE,
    ];
    const NAME: NameType = "snarkOS";
    const OPTIONS: &'static [OptionType] = &[
        option::IP,
        option::PORT,
        option::PATH,
        option::CONNECT,
        option::EXPORT_CANON_BLOCKS,
        option::IMPORT_CANON_BLOCKS,
        option::MINER_ADDRESS,
        option::MEMPOOL_INTERVAL,
        option::MIN_PEERS,
        option::MAX_PEERS,
        option::NETWORK,
        option::RPC_IP,
        option::RPC_PORT,
        option::RPC_USERNAME,
        option::RPC_PASSWORD,
        option::VERBOSE,
        option::MAX_HEAD,
    ];
    const SUBCOMMANDS: &'static [SubCommandType] = &[subcommand::UPDATE];

    /// Handle all CLI arguments and flags for skeleton node
    fn parse(arguments: &ArgMatches) -> Result<Self::Config, CliError> {
        let mut config = Config::read_config()?;
        config.parse(arguments, &[
            "network",
            "no-jsonrpc",
            "export-canon-blocks",
            "import-canon-blocks",
            "is-miner",
            "ip",
            "port",
            "path",
            "connect",
            "miner-address",
            "mempool-interval",
            "min-peers",
            "max-peers",
            "rpc-ip",
            "rpc-port",
            "rpc-username",
            "rpc-password",
            "trim-storage",
            "validate-storage",
            "no-recanonize",
            "verbose",
            "max-head",
        ]);

        if let ("update", Some(arguments)) = arguments.subcommand() {
            UpdateCLI::parse(arguments)?;
            std::process::exit(0x0100);
        }

        Ok(config)
    }
}
