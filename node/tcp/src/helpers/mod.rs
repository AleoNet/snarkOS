// Copyright (C) 2019-2022 Aleo Systems Inc.
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

mod config;
pub use config::Config;

pub mod connections;
pub use connections::{Connection, ConnectionSide};

mod known_peers;
pub use known_peers::KnownPeers;

mod stats;
pub use stats::Stats;

use tracing::{debug_span, error_span, info_span, trace_span, warn_span, Span};

// FIXME: this can probably be done more elegantly
/// Creates the Tcp's tracing span based on its name.
pub fn create_span(tcp_name: &str) -> Span {
    let mut span = trace_span!("tcp", name = tcp_name);
    if !span.is_disabled() {
        return span;
    } else {
        span = debug_span!("tcp", name = tcp_name);
    }
    if !span.is_disabled() {
        return span;
    } else {
        span = info_span!("tcp", name = tcp_name);
    }
    if !span.is_disabled() {
        return span;
    } else {
        span = warn_span!("tcp", name = tcp_name);
    }
    if !span.is_disabled() { span } else { error_span!("tcp", name = tcp_name) }
}
