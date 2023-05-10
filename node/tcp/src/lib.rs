// Copyright (C) 2019-2023 Aleo Systems Inc.
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

#![deny(missing_docs)]
#![deny(unsafe_code)]

//! **Tcp** is a simple, low-level, and customizable implementation of a TCP stack.

mod helpers;
pub use helpers::*;

pub mod protocols;

mod tcp;
pub use tcp::Tcp;

use std::net::IpAddr;

/// A trait for objects containing a [`Tcp`]; it is required to implement protocols.
pub trait P2P {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp;
}

/// Checks if the given IP address is a bogon address.
///
/// A bogon address is an IP address that should not appear on the public Internet.
/// This includes private addresses, loopback addresses, and link-local addresses.
pub fn is_bogon_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local(),
        IpAddr::V6(ipv6) => ipv6.is_loopback(),
    }
}
