use crate::{
    cli::CLI,
    parameters::{flag, option, types::*},
};
use snarkos_errors::node::CliError;

use clap::ArgMatches;
use serde::Serialize;

/// Hardcoded bootnodes maintained by Aleo.
/// A node should try and connect to these first after coming online.
pub const MAINNET_BOOTNODES: &'static [&str] = &[]; // "192.168.0.1:4130"
pub const TESTNET_BOOTNODES: &'static [&str] = &[]; // "192.168.0.1:4131"

/// Represents all configuration options for a node.
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    // Flags
    pub network: u8,
    pub jsonrpc: bool,
    pub is_bootnode: bool,
    pub is_miner: bool,
    pub quiet: bool,
    // Options
    pub ip: String,
    pub port: u16,
    pub path: String,
    pub bootnodes: Vec<String>,
    pub miner_address: String,
    pub mempool_interval: u8,
    pub min_peers: u16,
    pub max_peers: u16,

    pub rpc_port: u16,
    pub rpc_username: Option<String>,
    pub rpc_password: Option<String>,

    //Subcommand
    subcommand: Option<String>,
}

impl Default for Config {
    // TODO (raychu86) Parse from a snarkos config file
    fn default() -> Self {
        Self {
            // Flags
            network: 1,
            jsonrpc: true,
            is_bootnode: false,
            is_miner: true,
            quiet: false,
            // Options
            ip: "0.0.0.0".into(),
            port: 4131,
            path: "snarkos_testnet_1".into(),
            bootnodes: TESTNET_BOOTNODES
                .iter()
                .map(|node| (*node).to_string())
                .collect::<Vec<String>>(),
            miner_address: "aleo1faksgtpmculyzt6tgaq26fe4fgdjtwualyljjvfn2q6k42ydegzspfz9uh".into(),
            mempool_interval: 5,
            min_peers: 2,
            max_peers: 20,

            rpc_port: 3030,
            rpc_username: None,
            rpc_password: None,

            subcommand: None,
        }
    }
}

impl Config {
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
                    self.path = "snarkos_db".into();
                    self.port = 4130;
                    self.bootnodes = MAINNET_BOOTNODES
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();
                    self.network = network_id;
                }
                _ => {
                    self.path = format!("snarkos_testnet_{}", network_id);
                    self.port = 4130 + (network_id as u16);
                    self.bootnodes = TESTNET_BOOTNODES
                        .iter()
                        .map(|node| (*node).to_string())
                        .collect::<Vec<String>>();
                    self.network = network_id;
                }
            }
        }
    }

    fn no_jsonrpc(&mut self, argument: bool) {
        match argument {
            true => self.jsonrpc = false,
            false => self.jsonrpc = true,
        };
    }

    fn is_bootnode(&mut self, argument: bool) {
        self.is_bootnode = argument;
        if argument {
            self.bootnodes = vec![];
        }
    }

    fn is_miner(&mut self, argument: bool) {
        self.is_miner = argument;
    }

    fn quiet(&mut self, argument: bool) {
        self.quiet = argument;
    }

    fn ip(&mut self, argument: Option<&str>) {
        if let Some(ip) = argument {
            self.ip = ip.to_string();
        }
    }

    fn port(&mut self, argument: Option<u16>) {
        if let Some(port) = argument {
            self.port = port;
        }
    }

    fn path(&mut self, argument: Option<&str>) {
        match argument {
            Some(path) => self.path = path.into(),
            _ => (),
        };
    }

    fn connect(&mut self, argument: Option<&str>) {
        if let Some(bootnode) = argument {
            self.bootnodes = vec![bootnode.to_string()];
        }
    }

    fn miner_address(&mut self, argument: Option<&str>) {
        if let Some(miner_address) = argument {
            self.miner_address = miner_address.to_string();
        }
    }

    fn mempool_interval(&mut self, argument: Option<u8>) {
        if let Some(interval) = argument {
            self.mempool_interval = interval
        }
    }

    fn min_peers(&mut self, argument: Option<u16>) {
        if let Some(num_peers) = argument {
            self.min_peers = num_peers;
        }
    }

    fn max_peers(&mut self, argument: Option<u16>) {
        if let Some(num_peers) = argument {
            self.max_peers = num_peers;
        }
    }

    fn rpc_port(&mut self, argument: Option<u16>) {
        if let Some(rpc_port) = argument {
            self.rpc_port = rpc_port;
        }
    }

    fn rpc_username(&mut self, argument: Option<&str>) {
        if let Some(username) = argument {
            self.rpc_username = Some(username.to_string());
        }
    }

    fn rpc_password(&mut self, argument: Option<&str>) {
        if let Some(password) = argument {
            self.rpc_password = Some(password.to_string());
        }
    }
}

/// Parses command line arguments into node configuration parameters.
pub struct ConfigCli;

impl CLI for ConfigCli {
    type Config = Config;

    const ABOUT: AboutType = "Start an Aleo Node (include -h for more options)";
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
        let mut config = Config::default();
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
