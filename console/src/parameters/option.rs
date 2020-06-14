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

pub const GET_BLOCK: OptionType = (
    "[getblock] --getblock=[block_hash] 'Returns information about a block from a block hash'",
    &[],
    &[],
    &[],
);

pub const GET_BLOCK_COUNT: OptionType = (
    "[getblockcount] --getblockcount ' Returns the number of blocks in the canonical chain'",
    &[],
    &[],
    &[],
);

pub const GET_BEST_BLOCK_HASH: OptionType = (
    "[getbestblockhash] --getbestblockhash 'Returns the block hash of the head of the canonical chain'",
    &[],
    &[],
    &[],
);

pub const GET_BLOCK_HASH: OptionType = (
    "[getblockhash] --getblockhash=[block_number] 'Returns the block hash of the index specified if it exists in the canonical chain'",
    &[],
    &[],
    &[],
);

pub const GET_RAW_TRANSACTION: OptionType = (
    "[getrawtransaction] --getrawtransaction=[transaction_id] 'Returns hex encoded bytes of a transaction from its transaction id'",
    &[],
    &[],
    &[],
);

pub const GET_TRANSACTION_INFO: OptionType = (
    "[gettransactioninfo] --gettransactioninfo=[transaction_id] 'Returns information about a transaction from a transaction id'",
    &[],
    &[],
    &[],
);

pub const DECODE_RAW_TRANSACTION: OptionType = (
    "[decoderawtransaction] --decoderawtransaction=[transaction_bytes] 'Returns information about a transaction from serialized transaction bytes'",
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

pub const DECODE_RECORD: OptionType = (
    "[decoderecord] --decoderecord=[record_bytes] 'Returns information about a record from serialized record bytes'",
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
