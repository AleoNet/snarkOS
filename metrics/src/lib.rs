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

// The `test` and `prometheus` features are mutually exclusive, fail compilation if this occurs.
#[cfg(all(feature = "test", feature = "prometheus"))]
compile_error!("feature \"test\" and feature \"prometheus\" cannot be enabled at the same time");

mod names;

#[cfg(feature = "test")]
mod utils;

// Re-export the metrics macros.
pub use metrics::*;
pub use names::*;

// Re-export test utilities.
#[cfg(feature = "test")]
pub use utils::*;

#[cfg(feature = "test")]
use metrics_util::debugging::{DebuggingRecorder, Snapshotter};

#[cfg(feature = "prometheus")]
pub fn initialize() -> Option<tokio::task::JoinHandle<()>> {
    use metrics_exporter_prometheus::PrometheusBuilder;

    let (recorder, exporter) = PrometheusBuilder::new().build().expect("can't build the prometheus exporter");

    metrics::set_boxed_recorder(Box::new(recorder)).expect("can't set the prometheus exporter");

    let metrics_exporter_task = tokio::task::spawn(async move {
        exporter.await.expect("can't await the prometheus exporter");
    });

    register_metrics();

    Some(metrics_exporter_task)
}

#[cfg(feature = "test")]
pub fn initialize() -> Snapshotter {
    // Let the errors through in case the recorder has already been set as is likely when running
    // multiple metrics tests.
    let recorder = DebuggingRecorder::new();
    let snapshotter = recorder.snapshotter();
    let _ = recorder.install();

    register_metrics();

    snapshotter
}

fn register_metrics() {
    for name in GAUGES {
        register_gauge!(name);
    }

    for name in HISTOGRAMS {
        register_histogram!(name);
    }

    for name in COUNTERS {
        register_counter!(name);
    }
}
