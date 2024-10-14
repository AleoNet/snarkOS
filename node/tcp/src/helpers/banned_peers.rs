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

use std::{collections::HashMap, net::IpAddr, time::Instant};

use parking_lot::RwLock;
use tracing::trace;

/// Contains the ban details for a banned peer.
pub struct BanConfig {
    /// The time when the ban was created.
    banned_at: Instant,
    /// Amount of times the peer has been banned.
    banned_count: u8,
}

impl BanConfig {
    /// Creates a new ban config.
    pub fn new(count: u8) -> Self {
        Self { banned_at: Instant::now(), banned_count: count }
    }
}

/// Contains the set of peers currently banned by IP.
#[derive(Default)]
pub struct BannedPeers(RwLock<HashMap<IpAddr, BanConfig>>);

impl BannedPeers {
    /// Check whether the given IP address is currently banned.
    pub fn is_ip_banned(&self, ip: &IpAddr) -> bool {
        self.0.read().contains_key(ip)
    }

    /// Get ban count for the given IP address.
    pub fn get_ban_count(&self, ip: IpAddr) -> Option<u8> {
        self.0.read().get(&ip).map(|delay| delay.banned_count)
    }

    /// Insert or update a banned IP.
    pub fn update_ip_ban(&self, ip: IpAddr) {
        let count = self.get_ban_count(ip).unwrap_or(0).saturating_add(1u8);

        trace!("Banning IP: {:?} with count: {}", ip, count);
        let ban_config = BanConfig::new(count);
        self.0.write().insert(ip, ban_config);
    }

    /// Remove the expired entries
    pub fn remove_old_bans(&self, ban_time_in_secs: u64) {
        self.0.write().retain(|_, ban_config| {
            ban_config.banned_at.elapsed().as_secs() < (ban_time_in_secs << ban_config.banned_count.max(32))
        });
    }
}
