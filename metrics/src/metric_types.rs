use std::sync::atomic::{AtomicU64, Ordering};

/// Mimics a [`metrics-core`] monotonically increasing [`Counter`] type
pub struct Counter(AtomicU64);

impl Counter {
    pub(crate) const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Increases the value of the [`Counter`] by a discrete amount
    #[inline]
    pub(crate) fn increment(&self, val: u64) {
        self.0.fetch_add(val, Ordering::SeqCst);
    }

    /// Read the current state of the [`Counter`]
    #[inline]
    pub fn read(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Mimics a [`metrics-core`] arbitrarily increasing & decreasing [`Gauge`]
/// Limit granularity to discrete values, for real units, please use [`Gauge`]
pub struct DiscreteGauge(AtomicU64);

impl DiscreteGauge {
    pub(crate) const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Overwrite the value of the [`DiscreteGauge`] to a fixed discrete amount
    #[inline]
    pub(crate) fn set(&self, val: f64) {
        self.0.store(val as u64, Ordering::SeqCst);
    }

    /// Increases the value of the [`DiscreteGauge`] by a discrete amount
    #[inline]
    pub(crate) fn increase(&self, val: f64) {
        self.0.fetch_add(val as u64, Ordering::SeqCst);
    }

    /// Decreases the value of the [`DiscreteGauge`] by a discrete amount
    #[inline]
    pub(crate) fn decrease(&self, val: f64) {
        self.0.fetch_sub(val as u64, Ordering::SeqCst);
    }

    /// Read the current state of the [`DiscreteGauge`]
    #[inline]
    pub fn read(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

/// Mimics a [`metrics-core`] arbitrarily increasing & decreasing [`Gauge`]
/// Limit granularity to real values, for discrete units, please use [`DiscreteGauge`]
pub struct Gauge(AtomicU64);

#[allow(dead_code)]
impl Gauge {
    pub(crate) const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Overwrite the value of the [`Gauge`] to a fixed real amount
    #[inline]
    pub(crate) fn set(&self, val: f64) {
        self.0.store(val.to_bits(), Ordering::SeqCst);
    }

    /// Increases the value of the [`Gauge`] by a real amount
    #[inline]
    pub(crate) fn increase(&self, val: f64) {
        self.transform(|v| v + val);
    }

    /// Decreases the value of the [`Gauge`] by a real amount
    #[inline]
    pub(crate) fn decrease(&self, val: f64) {
        self.transform(|v| v - val);
    }

    /// Read the current state of the [`Gauge`]
    #[inline]
    pub fn read(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    /// Apply a numerical transformation to the [`f64`] interpretation of the stored value.
    /// Note: This is applied in a loop by a set of atomic compare-and-swap operations
    #[inline]
    fn transform<F: Fn(f64) -> f64>(&self, f: F) {
        let mut old = self.0.load(Ordering::Relaxed);
        loop {
            if let Err(previous) = self.0.compare_exchange_weak(
                f(f64::from_bits(old)).to_bits(),
                old,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                old = previous;
            } else {
                return;
            }
        }
    }
}
