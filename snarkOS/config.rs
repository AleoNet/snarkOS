use crate::{
    cli::CLI,
    parameters::{flag, option, types::*},
};
use snarkos_errors::node::CliError;
use snarkos_network::bootnodes::*;

use clap::ArgMatches;
use serde::Serialize;

/// Represents all configuration options for a node.
#[derive(Clone, Debug, Serialize)]
pub struct Config {
    // Flags
    pub network: String,
    pub jsonrpc: bool,
    pub is_bootnode: bool,
    pub miner: bool,
    pub quiet: bool,
    // Options
    pub ip: String,
    pub port: u16,
    pub path: String,
    pub rpc_port: u16,
    pub bootnodes: Vec<String>,
    pub miner_address: String,
    pub mempool_interval: u8,
    pub min_peers: u16,
    pub max_peers: u16,

    //Subcommand
    subcommand: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Flags
            network: "mainnet".into(),
            jsonrpc: true,
            is_bootnode: false,
            miner: true,
            quiet: false,
            // Options
            ip: "0.0.0.0".into(),
            port: 4130,
            path: "storage_db".into(),
            rpc_port: 3030,
            bootnodes: MAINNET_BOOTNODES
                .iter()
                .map(|node| (*node).to_string())
                .collect::<Vec<String>>(),
            miner_address: "90c0290b0913f0679ae6b27dde990a22863e14bced9125da7f446e5e953af900".into(),
            subcommand: None,
            mempool_interval: 5,
            min_peers: 2,
            max_peers: 20,
        }
    }
}

impl Config {
    fn parse(&mut self, arguments: &ArgMatches, options: &[&str]) {
        options.iter().for_each(|option| match *option {
            // Flags
            "network" => self.network(arguments.is_present(option)),
            "no_jsonrpc" => self.no_jsonrpc(arguments.is_present(option)),
            "is_bootnode" => self.is_bootnode(arguments.is_present(option)),
            "miner" => self.miner(arguments.is_present(option)),
            "quiet" => self.quiet(arguments.is_present(option)),
            // Options
            "ip" => self.ip(arguments.value_of(option)),
            "port" => self.port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "path" => self.path(arguments.value_of(option)),
            "rpc_port" => self.rpc_port(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "miner-address" => self.miner_address(arguments.value_of(option)),
            "connect" => self.connect(arguments.value_of(option)),
            "mempool_interval" => self.mempool_interval(clap::value_t!(arguments.value_of(*option), u8).ok()),
            "min_peers" => self.min_peers(clap::value_t!(arguments.value_of(*option), u16).ok()),
            "max_peers" => self.max_peers(clap::value_t!(arguments.value_of(*option), u16).ok()),
            _ => (),
        });
    }

    /// Sets `network` to the specified network, overriding its previous state.
    fn network(&mut self, argument: bool) {
        match argument {
            true => {
                self.path = "./skeleton_db_testnet".into();
                self.port = 18080;
                self.bootnodes = TESTNET_BOOTNODES
                    .iter()
                    .map(|node| (*node).to_string())
                    .collect::<Vec<String>>();
                self.network = "testnet".into();
            }
            false => self.network = "mainnet".into(),
        };
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

    fn miner(&mut self, argument: bool) {
        self.miner = argument;
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

    fn rpc_port(&mut self, argument: Option<u16>) {
        if let Some(rpc_port) = argument {
            self.rpc_port = rpc_port;
        }
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
}

/// Parses command line arguments into node configuration parameters.
pub struct ConfigCli;

impl CLI for ConfigCli {
    type Config = Config;

    const ABOUT: AboutType = "Start a skeleton blockchain miner (include -h for more options)";
    const FLAGS: &'static [FlagType] = &[
        flag::NETWORK,
        flag::NO_JSONRPC,
        flag::IS_BOOTNODE,
        flag::MINER,
        flag::QUIET,
    ];
    const NAME: NameType = "snarkos-node";
    const OPTIONS: &'static [OptionType] = &[
        option::IP,
        option::PORT,
        option::PATH,
        option::RPC_PORT,
        option::CONNECT,
        option::MINER_ADDRESS,
        option::MEMPOOL_INTERVAL,
        option::MIN_PEERS,
        option::MAX_PEERS,
    ];
    const SUBCOMMANDS: &'static [SubCommandType] = &[];

    /// Handle all CLI arguments and flags for skeleton node
    fn parse(arguments: &ArgMatches) -> Result<Self::Config, CliError> {
        let mut config = Config::default();
        config.parse(arguments, &[
            "network",
            "no_jsonrpc",
            "is_bootnode",
            "miner",
            "quiet",
            "ip",
            "port",
            "path",
            "rpc_port",
            "connect",
            "miner-address",
            "mempool_interval",
            "min_peers",
            "max_peers",
        ]);

        Ok(config)
    }
}
