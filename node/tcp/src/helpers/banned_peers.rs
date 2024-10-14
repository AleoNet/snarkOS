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

/// Contains the ban details for a banned peer.
#[derive(Clone)]
pub struct BanConfig {
    /// The time when the ban was created.
    banned_at: Instant,
    /// Amount of times the peer has been banned.
    count: u8,
}

impl BanConfig {
    /// Creates a new ban config.
    pub fn new(banned_at: Instant, count: u8) -> Self {
        Self { banned_at, count }
    }

    pub fn update_count(&mut self) {
        self.count += 1;
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
        self.0.read().get(&ip).map(|delay| delay.count)
    }

    /// Get all banned IPs.
    pub fn get_banned_ips(&self) -> Vec<IpAddr> {
        self.0.read().keys().cloned().collect()
    }

    /// Get ban config for the given IP address.
    pub fn get_ban_config(&self, ip: IpAddr) -> Option<BanConfig> {
        self.0.read().get(&ip).cloned()
    }

    /// Insert or update a banned IP.
    pub fn update_ip_ban(&self, ip: IpAddr) {
        if let Some(config) = self.get_ban_config(ip) {
            let mut config = config;
            config.update_count();
            self.0.write().insert(ip, config);
        } else {
            self.0.write().insert(ip, BanConfig::new(Instant::now(), 1u8));
        }
    }

    /// Remove the expired entries
    pub fn remove_old_bans(&self, ban_time_in_secs: u64) {
        self.0.write().retain(|_, ban_config| {
            let shift = ban_config.count.min(32);
            let ban_duration = ban_time_in_secs.checked_shl(shift as u32).unwrap_or(u64::MAX);
            ban_config.banned_at.elapsed().as_secs() < ban_duration
        });
    }
}
