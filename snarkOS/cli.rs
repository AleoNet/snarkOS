use crate::parameters::types::*;
use snarkos_errors::node::CliError;

use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};

pub trait CLI {
    type Config;

    const NAME: NameType;
    const ABOUT: AboutType;
    const FLAGS: &'static [FlagType];
    const OPTIONS: &'static [OptionType];
    const SUBCOMMANDS: &'static [SubCommandType];

    #[cfg_attr(tarpaulin, skip)]
    fn new<'a>() -> ArgMatches<'a> {
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
            .version("0.1.0")
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
