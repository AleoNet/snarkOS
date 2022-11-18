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

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Contains statistics related to Tcp.
#[derive(Default)]
pub struct Stats {
    /// The number of all messages sent.
    msgs_sent: AtomicU64,
    /// The number of all messages received.
    msgs_received: AtomicU64,
    /// The number of all bytes sent.
    bytes_sent: AtomicU64,
    /// The number of all bytes received.
    bytes_received: AtomicU64,
    /// The number of failures.
    failures: AtomicU64,
}

impl Stats {
    /// Returns the number of sent messages and their collective size in bytes.
    pub fn sent(&self) -> (u64, u64) {
        let msgs = self.msgs_sent.load(Relaxed);
        let bytes = self.bytes_sent.load(Relaxed);

        (msgs, bytes)
    }

    /// Returns the number of received messages and their collective size in bytes.
    pub fn received(&self) -> (u64, u64) {
        let msgs = self.msgs_received.load(Relaxed);
        let bytes = self.bytes_received.load(Relaxed);

        (msgs, bytes)
    }

    /// Returns the number of failures.
    pub fn failures(&self) -> u64 {
        self.failures.load(Relaxed)
    }

    /// Registers a sent message of the provided `size` in bytes.
    pub fn register_sent_message(&self, size: usize) {
        self.msgs_sent.fetch_add(1, Relaxed);
        self.bytes_sent.fetch_add(size as u64, Relaxed);
    }

    /// Registers a received message of the provided `size` in bytes.
    pub fn register_received_message(&self, size: usize) {
        self.msgs_received.fetch_add(1, Relaxed);
        self.bytes_received.fetch_add(size as u64, Relaxed);
    }

    /// Registers a failure.
    pub fn register_failure(&self) {
        self.failures.fetch_add(1, Relaxed);
    }
}
