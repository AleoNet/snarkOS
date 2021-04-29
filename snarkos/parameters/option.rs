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
    "[port] -p --port=[port] 'Specify the port the node is run on'",
    &[],
    &[],
    &[],
);

pub const CONNECT: OptionType = (
    "[connect] --connect=[ip] 'Specify one or more node ip addresses to connect to on startup'",
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
    "[mempool-interval] --mempool-interval=[mempool-interval] 'Specify the frequency in seconds the node should fetch a sync node's mempool'",
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

pub const NETWORK: OptionType = (
    "[network] --network=[network-id] 'Specify the network id (default = 1) of the node'",
    &[],
    &[],
    &[],
);

pub const RPC_IP: OptionType = (
    "[rpc-ip] --rpc-ip=[rpc-ip] 'Specify the ip of the RPC server'",
    &[],
    &[],
    &[],
);

pub const RPC_PORT: OptionType = (
    "[rpc-port] --rpc-port=[rpc-port] 'Specify the port the json rpc server is run on'",
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

pub const VERBOSE: OptionType = (
    "[verbose] --verbose=[verbose] 'Specify the verbosity (default = 1) of the node'",
    &[],
    &["0", "1", "2", "3", "4"],
    &[],
);
