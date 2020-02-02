use crate::types::*;

// Format
// (argument, conflicts, possible_values, requires)

// Global

pub const IP: OptionType = ("[ip] -i --ip=[ip] 'Specify the ip of your node'", &[], &[], &[]);

pub const PORT: OptionType = (
    "[port] -p --port=[port] 'Connect to an RPC server with a specified port'",
    &[],
    &[],
    &[],
);

pub const GET_BALANCE: OptionType = (
    "[getbalance] --getbalance=[address] 'Returns the total available balance for an address'",
    &[],
    &[],
    &[],
);

pub const GET_BLOCK: OptionType = (
    "[getblock] --getblock=[block_hash] 'Returns a block for a particular block header hash'",
    &[],
    &[],
    &[],
);

pub const GET_BLOCK_COUNT: OptionType = (
    "[getblockcount] --getblockcount 'Returns the number of blocks in the longest chain'",
    &[],
    &[],
    &[],
);

pub const GET_BEST_BLOCK_HASH: OptionType = (
    "[getbestblockhash] --getbestblockhash 'Returns the hash of the best block in the longest chain'",
    &[],
    &[],
    &[],
);

pub const LIST_UNSPENT: OptionType = (
    "[listunspent] --listunspent=[address] 'Returns a list of unspent transaction outputs for an address'",
    &[],
    &[],
    &[],
);

pub const GET_RAW_TRANSACTION: OptionType = (
    "[getrawtransaction] --getrawtransaction=[transaction_id] 'Returns the transaction information for a transaction id'",
    &[],
    &[],
    &[],
);

pub const CREATE_RAW_TRANSACTION: OptionType = (
    "[createrawtransaction] --createrawtransaction= [inputs] [outputs] 'Generates a raw transaction
    Inputs format: '[{\"txid\":\"txid\", \"vout\":index},...]'
    Outputs format: '{\"address\":amount,...}''",
    &[],
    &[],
    &[],
);

pub const DECODE_RAW_TRANSACTION: OptionType = (
    "[decoderawtransaction] --decoderawtransaction=[transaction_bytes] 'Returns a decoded transaction from a serialized transaction'",
    &[],
    &[],
    &[],
);

pub const SIGN_RAW_TRANSACTION: OptionType = (
    "[signrawtransaction] --signrawtransaction=[transaction_bytes][private_keys] 'Sign a raw transaction Private keys format: '[\"private_key\", ...]''",
    &[],
    &[],
    &[],
);

pub const SEND_RAW_TRANSACTION: OptionType = (
    "[sendrawtransaction] --sendrawtransaction=[transaction_bytes] 'Broadcast a raw transaction'",
    &[],
    &[],
    &[],
);

pub const GET_CONNECTION_COUNT: OptionType = (
    "[getconnectioncount] --getconnectioncount 'Returns the number of peers the node is connected to'",
    &[],
    &[],
    &[],
);

pub const GET_PEER_INFO: OptionType = (
    "[getpeerinfo] --getpeerinfo 'Returns data about each connected network node'",
    &[],
    &[],
    &[],
);

pub const GET_BLOCK_TEMPLATE: OptionType = (
    "[getblocktemplate] --getblocktemplate 'Returns a block template for mining purposes'",
    &[],
    &[],
    &[],
);
