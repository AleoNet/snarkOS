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

use std::sync::atomic::{AtomicU32, AtomicU64};

#[derive(Default)]
pub struct Stats {
    /// The monotonic counter for the number of send requests that succeeded.
    pub send_success_count: AtomicU64,
    /// The monotonic counter for the number of send requests that failed.
    pub send_failure_count: AtomicU64,
    /// The number of successfully processed inbound messages.
    pub recv_success_count: AtomicU64,
    /// The number of inbound messages that couldn't be processed.
    pub recv_failure_count: AtomicU64,
    /// The current number of items in the inbound channel.
    pub inbound_channel_items: AtomicU64,
    /// The number of all connection requests the node has received.
    pub inbound_connection_requests: AtomicU64,
    /// The number of outbound connection requests.
    pub outbound_connection_requests: AtomicU64,
    /// The number of mined blocks.
    pub blocks_mined: AtomicU32,
}
