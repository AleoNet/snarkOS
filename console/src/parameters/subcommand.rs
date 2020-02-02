use crate::{option, types::*};

use clap::AppSettings;

// Format
// (name, about, options, settings)

pub const TEST_SUBCOMMAND: SubCommandType = (
    "test",
    "testing purposes only (to get around default cargo run)",
    &[
        option::IP,
        option::PORT,
        option::GET_BALANCE,
        option::GET_BLOCK,
        option::GET_BLOCK_COUNT,
        option::GET_BEST_BLOCK_HASH,
        option::LIST_UNSPENT,
        option::GET_RAW_TRANSACTION,
        option::CREATE_RAW_TRANSACTION,
        option::DECODE_RAW_TRANSACTION,
        option::SIGN_RAW_TRANSACTION,
        option::SEND_RAW_TRANSACTION,
        option::GET_CONNECTION_COUNT,
        option::GET_PEER_INFO,
        option::GET_BLOCK_TEMPLATE,
    ],
    &[
        AppSettings::ColoredHelp,
        //        AppSettings::DisableHelpSubcommand,
        AppSettings::DisableVersion,
    ],
);
