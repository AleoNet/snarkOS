use std::sync::atomic::{AtomicU64, Ordering};

/// Mimics a [`metrics-core`] monotonically increasing [`Counter`] type
pub struct Counter(AtomicU64);

impl Counter {
    pub(crate) const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub(crate) fn increment(&self, val: u64) {
        self.0.fetch_add(val, Ordering::SeqCst);
    }
}

/// Mimics a [`metrics-core`] arbitrarily increasing & decreasing [`Gauge`]
pub struct Gauge(AtomicU64);

impl Gauge {
    pub(crate) const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub(crate) fn set(&self, val: f64) {
        // TODO: @sadroeck - Reinterpret as f64 & do atomic C&S
        self.0.store(val as u64, Ordering::SeqCst);
    }

    pub(crate) fn increase(&self, val: f64) {
        // TODO: @sadroeck - Reinterpret as f64 & do atomic C&S
        self.0.fetch_add(val as u64, Ordering::SeqCst);
    }

    pub(crate) fn decrease(&self, val: f64) {
        // TODO: @sadroeck - Reinterpret as f64 & do atomic C&S
        self.0.fetch_sub(val as u64, Ordering::SeqCst);
    }
}
