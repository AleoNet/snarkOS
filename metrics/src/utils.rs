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

use metrics::Key;
use metrics_util::{
    debugging::{DebugValue, Snapshotter},
    CompositeKey,
    MetricKind,
};

pub struct TestMetrics(Snapshotter);

impl Default for TestMetrics {
    fn default() -> Self {
        Self(crate::initialize())
    }
}

impl TestMetrics {
    pub fn get_val_for(&self, metric: &'static str) -> MetricVal {
        let key = CompositeKey::new(MetricKind::Gauge, Key::from_name(metric));

        match &self.0.snapshot().into_hashmap().get(&key).unwrap().2 {
            DebugValue::Gauge(val) => MetricVal::Gauge(val.into_inner()),
            DebugValue::Counter(val) => MetricVal::Counter(*val),
            DebugValue::Histogram(vals) => MetricVal::Histogram(vals.iter().map(|val| val.into_inner()).collect()),
        }
    }
}

impl Drop for TestMetrics {
    fn drop(&mut self) {
        // Clear the recorder to avoid the global state bleeding into other tests.
        metrics::clear_recorder();
    }
}

#[derive(Debug, PartialEq)]
pub enum MetricVal {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
}
