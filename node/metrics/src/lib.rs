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

mod names;

// Expose the names at the crate level for easy access.
pub use names::*;
// Re-export the snarkVM metrics.
pub use snarkvm::metrics::*;

/// Initializes the metrics and returns a handle to the task running the metrics exporter.
pub fn initialize_metrics() {
    // Build the Prometheus exporter.
    metrics_exporter_prometheus::PrometheusBuilder::new().install().expect("can't build the prometheus exporter");

    // Register the snarkVM metrics.
    snarkvm::metrics::register_metrics();

    // Register the metrics so they exist on init.
    for name in crate::names::GAUGE_NAMES {
        register_gauge(name);
    }
    for name in crate::names::COUNTER_NAMES {
        register_counter(name);
    }
    for name in crate::names::HISTOGRAM_NAMES {
        register_histogram(name);
    }
}
