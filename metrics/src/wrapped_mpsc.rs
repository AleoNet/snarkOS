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

use std::{
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use tokio::sync::mpsc::{
    self,
    error::{SendError, TrySendError},
};

/// Wrapper over mpsc::Sender to track metrics
pub struct Sender<T: Send> {
    inner: mpsc::Sender<T>,
    tracker: Arc<AtomicUsize>,
    metrics_tracker: &'static str,
}

impl<T: Send> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            tracker: self.tracker.clone(),
            metrics_tracker: self.metrics_tracker,
        }
    }
}

impl<T: Send> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "sender for {}", self.metrics_tracker)
    }
}

impl<T: Send> Sender<T> {
    fn increment(&self) {
        metrics::increment_gauge!(self.metrics_tracker, 1.0);
        self.tracker.fetch_add(1, Ordering::SeqCst);
    }

    fn maybe_decrement(&self) {
        if self.tracker.load(Ordering::SeqCst) != 0 {
            metrics::decrement_gauge!(self.metrics_tracker, 1.0);
            self.tracker.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub async fn send(&self, value: T) -> Result<(), SendError<T>> {
        self.increment();
        let ret = self.inner.send(value).await;
        if ret.is_err() {
            self.maybe_decrement();
        }
        ret
    }

    pub fn try_send(&self, message: T) -> Result<(), TrySendError<T>> {
        self.inner.try_send(message)?;
        self.increment();
        Ok(())
    }

    pub fn blocking_send(&self, value: T) -> Result<(), SendError<T>> {
        self.increment();
        let ret = self.inner.blocking_send(value);
        if ret.is_err() {
            self.maybe_decrement();
        }
        ret
    }
}

/// Wrapper over mpsc::Receiver to track metrics
#[derive(Debug)]
pub struct Receiver<T: Send> {
    inner: mpsc::Receiver<T>,
    tracker: Arc<AtomicUsize>,
    metrics_tracker: &'static str,
}

impl<T: Send> Receiver<T> {
    fn maybe_decrement(&self, is_ok: bool) {
        if is_ok {
            metrics::decrement_gauge!(self.metrics_tracker, 1.0);
            self.tracker.fetch_sub(1, Ordering::SeqCst);
        }
    }

    pub async fn recv(&mut self) -> Option<T> {
        let out = self.inner.recv().await;
        self.maybe_decrement(out.is_some());
        out
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        let out = self.inner.blocking_recv();
        self.maybe_decrement(out.is_some());
        out
    }
}

pub fn channel<T: Send>(metrics_tracker: &'static str, buffer: usize) -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = mpsc::channel(buffer);
    let tracker = Arc::new(AtomicUsize::new(0));

    (
        Sender {
            inner: sender,
            metrics_tracker,
            tracker: tracker.clone(),
        },
        Receiver {
            inner: receiver,
            metrics_tracker,
            tracker,
        },
    )
}

impl<T: Send> Drop for Receiver<T> {
    fn drop(&mut self) {
        let count = self.tracker.swap(0, Ordering::SeqCst);
        if count != 0 {
            metrics::decrement_gauge!(self.metrics_tracker, count as f64);
        }
    }
}
