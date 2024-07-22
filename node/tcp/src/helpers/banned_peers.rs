// Copyright (C) 2019-2023 Aleo Systems Inc.
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

/// Contains the set of peers currently banned by IP.
#[derive(Default)]
pub struct BannedPeers(RwLock<HashMap<IpAddr, Instant>>);

impl BannedPeers {
    /// Check whether the given IP address is currently banned.
    pub fn is_ip_banned(&self, ip: IpAddr) -> bool {
        self.0.read().contains_key(&ip)
    }

    /// Insert or update a banned IP.
    pub fn update_ip_ban(&self, ip: IpAddr) {
        let timestamp = Instant::now();
        self.0.write().insert(ip, timestamp);
    }

    /// Remove the expired entries.
    pub fn remove_old_bans(&self, ban_time_in_secs: u64) {
        self.0.write().retain(|_, timestamp| timestamp.elapsed().as_secs() < ban_time_in_secs);
    }
}
