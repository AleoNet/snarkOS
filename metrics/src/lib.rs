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

mod metric_types;
mod names;

pub use names::*;

pub mod snapshots;
pub mod stats;

/// Re-export metrics macros
pub use metrics::*;

pub struct CombinedRecorder {
    #[cfg(feature = "prometheus")]
    prometheus: metrics_exporter_prometheus::PrometheusRecorder,
    rpc: &'static stats::Stats,
}

impl Recorder for CombinedRecorder {
    fn register_counter(&self, key: &Key, unit: Option<Unit>, desc: Option<&'static str>) {
        #[cfg(feature = "prometheus")]
        self.prometheus.register_counter(key, unit.clone(), desc);
        self.rpc.register_counter(key, unit, desc);
    }

    fn register_gauge(&self, key: &Key, unit: Option<Unit>, desc: Option<&'static str>) {
        #[cfg(feature = "prometheus")]
        self.prometheus.register_gauge(key, unit.clone(), desc);
        self.rpc.register_gauge(key, unit, desc);
    }

    fn register_histogram(&self, key: &Key, unit: Option<Unit>, desc: Option<&'static str>) {
        #[cfg(feature = "prometheus")]
        self.prometheus.register_histogram(key, unit.clone(), desc);
        self.rpc.register_histogram(key, unit, desc);
    }

    fn record_histogram(&self, key: &Key, value: f64) {
        #[cfg(feature = "prometheus")]
        self.prometheus.record_histogram(key, value);
        self.rpc.record_histogram(key, value);
    }

    fn increment_counter(&self, key: &Key, value: u64) {
        #[cfg(feature = "prometheus")]
        self.prometheus.increment_counter(key, value);
        self.rpc.increment_counter(key, value);
    }

    fn update_gauge(&self, key: &Key, value: GaugeValue) {
        #[cfg(feature = "prometheus")]
        self.prometheus.update_gauge(key, value.clone());
        self.rpc.update_gauge(key, value);
    }
}

#[cfg(feature = "prometheus")]
pub fn initialize() -> Option<tokio::task::JoinHandle<()>> {
    let prometheus_builder = metrics_exporter_prometheus::PrometheusBuilder::new();

    let (prometheus_recorder, exporter) = prometheus_builder
        .build_with_exporter()
        .expect("can't build the prometheus exporter");

    let recorder = CombinedRecorder {
        prometheus: prometheus_recorder,
        rpc: &stats::NODE_STATS,
    };

    metrics::set_boxed_recorder(Box::new(recorder)).expect("can't set the prometheus exporter");

    let metrics_exporter_task = tokio::task::spawn(async move {
        exporter.await.expect("can't await the prometheus exporter");
    });

    Some(metrics_exporter_task)
}

#[cfg(not(feature = "prometheus"))]
pub fn initialize() -> Option<tokio::task::JoinHandle<()>> {
    metrics::set_recorder(&crate::stats::NODE_STATS).expect("couldn't initialize the metrics recorder!");

    None
}
