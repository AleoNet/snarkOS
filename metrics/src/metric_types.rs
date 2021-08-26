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

use circular_queue::CircularQueue;
use once_cell::sync::OnceCell;
use parking_lot::RwLock;

const QUEUE_CAPACITY: usize = 256;

/// A histogram backed by a circular queue.
pub struct CircularHistogram(OnceCell<RwLock<CircularQueue<f64>>>);

#[allow(dead_code)]
impl CircularHistogram {
    pub(crate) const fn new() -> Self {
        // The cell allows the creation of the object in a const fn.
        Self(OnceCell::new())
    }

    /// Push the value into the queue.
    #[inline]
    pub(crate) fn push(&self, val: f64) {
        self.0
            .get_or_init(|| RwLock::new(CircularQueue::with_capacity(QUEUE_CAPACITY)))
            .write()
            .push(val);
    }

    /// Computes the average over the stored values in the queue.
    #[inline]
    pub(crate) fn average(&self) -> f64 {
        let queue_r = self
            .0
            .get_or_init(|| RwLock::new(CircularQueue::with_capacity(QUEUE_CAPACITY)))
            .read();

        let sum: f64 = queue_r.iter().copied().sum();

        sum / queue_r.len() as f64
    }
}
