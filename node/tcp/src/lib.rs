// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
pub fn is_bogon_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local(),
        IpAddr::V6(ipv6) => ipv6.is_loopback(),
    }
}

/// Checks if the given IP address is unspecified or broadcast.
pub fn is_unspecified_or_broadcast_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => ipv4.is_unspecified() || ipv4.is_broadcast(),
        ipv6 => ipv6.is_unspecified(),
    }
}
