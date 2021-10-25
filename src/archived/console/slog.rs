//! `slog` support for `tui-logger`

use crate::console::display::TUI_LOGGER;

use log::{self, Log, Record};
use slog::{self, Drain, KV};
use std::{fmt, io};

/// Key-Separator-Value serializer
// Copied from `slog-stdlog`
struct KSV<W: io::Write> {
    io: W,
}

impl<W: io::Write> KSV<W> {
    fn new(io: W) -> Self {
        KSV { io }
    }

    fn into_inner(self) -> W {
        self.io
    }
}

impl<W: io::Write> slog::Serializer for KSV<W> {
    fn emit_arguments(&mut self, key: slog::Key, val: &fmt::Arguments) -> slog::Result {
        write!(self.io, ", {}: {}", key, val)?;
        Ok(())
    }
}

// Copied from `slog-stdlog`
struct LazyLogString<'a> {
    info: &'a slog::Record<'a>,
    logger_values: &'a slog::OwnedKVList,
}

impl<'a> LazyLogString<'a> {
    fn new(info: &'a slog::Record, logger_values: &'a slog::OwnedKVList) -> Self {
        LazyLogString {
            info,
            logger_values,
        }
    }
}

impl<'a> fmt::Display for LazyLogString<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.info.msg())?;

        let io = io::Cursor::new(Vec::new());
        let mut ser = KSV::new(io);

        self.logger_values
            .serialize(self.info, &mut ser)
            .map_err(|_| fmt::Error)?;
        self.info
            .kv()
            .serialize(self.info, &mut ser)
            .map_err(|_| fmt::Error)?;

        let values = ser.into_inner().into_inner();

        write!(f, "{}", String::from_utf8_lossy(&values))
    }
}

#[allow(clippy::needless_doctest_main)]
///  slog-compatible Drain that feeds messages to `tui-logger`.
///
///  ## Basic usage:
///  ```
///  use slog::{self, o, Drain, info};
///  //use tui_logger;
///
///  fn main() {
///     let drain = tui_logger::slog_drain().fuse();
///     let log = slog::Logger::root(drain, o!());
///     info!(log, "Logging via slog works!");
///
///  }
pub struct TuiSlogDrain;

impl Drain for TuiSlogDrain {
    type Ok = ();
    type Err = io::Error;
    // Copied from `slog-stdlog`
    fn log(&self, info: &slog::Record, logger_values: &slog::OwnedKVList) -> io::Result<()> {
        let level = match info.level() {
            slog::Level::Critical | slog::Level::Error => log::Level::Error,
            slog::Level::Warning => log::Level::Warn,
            slog::Level::Info => log::Level::Info,
            slog::Level::Debug => log::Level::Debug,
            slog::Level::Trace => log::Level::Trace,
        };

        let mut target = info.tag();
        if target.is_empty() {
            target = info.module();
        }

        let lazy = LazyLogString::new(info, logger_values);
        TUI_LOGGER.log(
            &Record::builder()
                .args(format_args!("{}", lazy))
                .level(level)
                .target(target)
                .file(Some(info.file()))
                .line(Some(info.line()))
                .build(),
        );

        Ok(())
    }
}