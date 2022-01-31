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

// The details on resource-limiting can be found at https://github.com/paritytech/jsonrpsee/blob/master/core/src/server/resource_limiting.rs

// note: jsonrpsee expects string literals as resource names; we'll be distinguishing
// them by the const name, so in order for the actual lookups to be faster, we can make
// the underlying strings short, as long as they are unique.

/// The resource label corresponding to the number of all active RPC calls.
pub(crate) const ALL_CONCURRENT_REQUESTS: &str = "0";
/// The maximum number of RPC requests that can be handled at once at any given time.
pub(crate) const ALL_CONCURRENT_REQUESTS_LIMIT: u16 = 10;
