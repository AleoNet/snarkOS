use crate::parameters::{option, types::*};

use clap::AppSettings;

// Format
// (name, about, options, settings)

pub const TEST_SUBCOMMAND: SubCommandType = (
    "test",
    "testing purposes only (to get around default cargo run)",
    &[
        option::IP,
        option::PORT,
        option::PATH,
        option::RPC_PORT,
        option::CONNECT,
        option::COINBASE_ADDRESS,
        option::MEMPOOL_INTERVAL,
        option::MIN_PEERS,
        option::MAX_PEERS,
    ],
    &[
        AppSettings::ColoredHelp,
        //        AppSettings::DisableHelpSubcommand,
        //        AppSettings::DisableVersion,
    ],
);
