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

use crate::{errors::CliError, parameters::types::*};

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

pub trait CLI {
    type Config;

    const NAME: NameType;
    const ABOUT: AboutType;
    const FLAGS: &'static [FlagType];
    const OPTIONS: &'static [OptionType];
    const SUBCOMMANDS: &'static [SubCommandType];

    #[cfg_attr(tarpaulin, skip)]
    fn args<'a>() -> ArgMatches<'a> {
        let flags = &Self::FLAGS
            .iter()
            .map(|a| Arg::from_usage(a))
            .collect::<Vec<Arg<'static, 'static>>>();
        let options = &Self::OPTIONS
            .iter()
            .map(|a| match !a.2.is_empty() {
                true => Arg::from_usage(a.0)
                    .conflicts_with_all(a.1)
                    .possible_values(a.2)
                    .requires_all(a.3),
                false => Arg::from_usage(a.0).conflicts_with_all(a.1).requires_all(a.3),
            })
            .collect::<Vec<Arg<'static, 'static>>>();
        let subcommands = Self::SUBCOMMANDS
            .iter()
            .map(|s| {
                SubCommand::with_name(s.0)
                    .about(s.1)
                    .args(
                        &s.2.iter()
                            .map(|a| match !a.2.is_empty() {
                                true => Arg::from_usage(a.0)
                                    .conflicts_with_all(a.1)
                                    .possible_values(a.2)
                                    .requires_all(a.3),
                                false => Arg::from_usage(a.0).conflicts_with_all(a.1).requires_all(a.3),
                            })
                            .collect::<Vec<Arg<'static, 'static>>>(),
                    )
                    .args(
                        &s.3.iter()
                            .map(|a| Arg::from_usage(a))
                            .collect::<Vec<Arg<'static, 'static>>>(),
                    )
                    .settings(s.4)
            })
            .collect::<Vec<App<'static, 'static>>>();

        App::new(Self::NAME)
            .about(Self::ABOUT)
            .version("1.1.4")// TODO (raychu86) Add auto fill for crate version
            .settings(&[
                AppSettings::ColoredHelp,
                //                AppSettings::DisableHelpSubcommand,
                //                AppSettings::ArgRequiredElseHelp,
                AppSettings::DisableVersion,
            ])
            .args(flags)
            .args(options)
            .subcommands(subcommands)
            .set_term_width(0)
            .get_matches()
    }

    #[cfg_attr(tarpaulin, skip)]
    fn parse(arguments: &ArgMatches) -> Result<Self::Config, CliError>;
}
