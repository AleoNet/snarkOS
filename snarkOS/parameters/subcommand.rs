use crate::parameters::{flag, types::*};

use clap::AppSettings;

// Format
// (name, about, options, flags, settings)

pub const UPDATE: SubCommandType = (
    "update",
    "Update the snarkOS to the latest version (include -h for more options)",
    &[],
    &[flag::LIST],
    &[
        AppSettings::ColoredHelp,
        AppSettings::DisableHelpSubcommand,
        AppSettings::DisableVersion,
    ],
);
