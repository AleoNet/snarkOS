// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use time::{
    format_description::{self, OwnedFormatItem},
    OffsetDateTime,
};
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{format::Writer, FmtContext, FormatEvent, FormatFields},
    registry::LookupSpan,
};

pub struct DynamicFormatter {
    dim_format: DimFormat,
    default_format: tracing_subscriber::fmt::format::Format,
    dim: Arc<AtomicBool>,
}

impl<S, N> FormatEvent<S, N> for DynamicFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, writer: Writer<'_>, event: &Event<'_>) -> std::fmt::Result {
        if self.dim.load(Ordering::Relaxed) {
            self.dim_format.format_event(ctx, writer, event)
        } else {
            self.default_format.format_event(ctx, writer, event)
        }
    }
}

impl DynamicFormatter {
    pub fn new(dim: Arc<AtomicBool>) -> Self {
        let dim_format = DimFormat::new();
        let default_format = tracing_subscriber::fmt::format::Format::default();
        Self { dim_format, default_format, dim }
    }
}

struct DimFormat {
    fmt: OwnedFormatItem,
}

impl DimFormat {
    fn new() -> Self {
        let format =
            format_description::parse_owned::<2>("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z")
                .expect("failed to set timestampt format");
        Self { fmt: format }
    }
}

impl<S, N> FormatEvent<S, N> for DimFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, S, N>, mut writer: Writer<'_>, event: &Event<'_>) -> std::fmt::Result {
        // set the DIM style
        if writer.has_ansi_escapes() {
            write!(writer, "\x1b[2m")?;
        }

        let date_time = OffsetDateTime::now_utc();
        write!(writer, "{}  ", date_time.format(&self.fmt).map_err(|_| std::fmt::Error)?)?;

        let meta = event.metadata();
        let fmt_level = match *meta.level() {
            tracing::Level::ERROR => "ERROR",
            tracing::Level::WARN => "WARN ",
            tracing::Level::INFO => "INFO ",
            tracing::Level::DEBUG => "DEBUG",
            tracing::Level::TRACE => "TRACE",
        };
        write!(writer, "{}", fmt_level)?;

        write!(writer, "{}: ", meta.target())?;

        ctx.format_fields(writer.by_ref(), event)?;

        // reset the style
        if writer.has_ansi_escapes() {
            write!(writer, "\x1b[0m")?;
        }
        writeln!(writer)
    }
}
