use crate::parameters::types::*;

// Format
// (argument, conflicts, possible_values, requires)

// Global

pub const PATH: OptionType = (
    "[path] -d --path=[path] 'Specify the node's storage path'",
    &[],
    &[],
    &[],
);

pub const IP: OptionType = ("[ip] -i --ip=[ip] 'Specify the ip of your node'", &[], &[], &[]);

pub const PORT: OptionType = (
    "[port] -p --port=[port] 'Run the node on a specified port'",
    &[],
    &[],
    &[],
);

pub const CONNECT: OptionType = (
    "[connect] --connect=[ip] 'Specify a node ip address to connect to on startup'",
    &[],
    &[],
    &[],
);

pub const MINER_ADDRESS: OptionType = (
    "[miner-address] --miner-address=[miner-address] 'Specify the address that will receive miner rewards'",
    &[],
    &[],
    &[],
);

pub const MEMPOOL_INTERVAL: OptionType = (
    "[mempool-interval] --mempool-interval=[mempool-interval] 'Specify the frequency in seconds x 10 the node should fetch the mempool from sync node'",
    &[],
    &[],
    &[],
);

pub const MIN_PEERS: OptionType = (
    "[min-peers] --min-peers=[min-peers] 'Specify the minimum number of peers the node should connect to'",
    &[],
    &[],
    &[],
);

pub const MAX_PEERS: OptionType = (
    "[max-peers] --max-peers=[max-peers] 'Specify the maximum number of peers the node can connect to'",
    &[],
    &[],
    &[],
);

pub const RPC_PORT: OptionType = (
    "[rpc-port] --rpc-port=[rpc-port] 'Run the rpc server on a specified port'",
    &["no_jsonrpc"],
    &[],
    &[],
);

pub const RPC_USERNAME: OptionType = (
    "[rpc-username] --rpc-username=[rpc-username] 'Specify a username for rpc authentication'",
    &["no-jsonrpc"],
    &[],
    &["rpc-password"],
);

pub const RPC_PASSWORD: OptionType = (
    "[rpc-password] --rpc-password=[rpc-password] 'Specify a password for rpc authentication'",
    &["no-jsonrpc"],
    &[],
    &["rpc-username"],
);
