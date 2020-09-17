// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::outbound::Channel;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

/// Stores connected peers and the channels for reading/writing messages to them.
pub struct Connections {
    channels: HashMap<SocketAddr, Arc<Channel>>,
}

impl Connections {
    /// Construct new store of peer `Connections`.
    pub fn new() -> Self {
        Connections {
            channels: HashMap::<SocketAddr, Arc<Channel>>::new(),
        }
    }

    /// Returns the channel stored at address if any.
    pub fn get(&self, address: &SocketAddr) -> Option<Arc<Channel>> {
        self.channels.get(address).cloned()
    }

    /// Stores a new channel at the peer address it is connected to.
    pub fn store_channel(&mut self, channel: &Arc<Channel>) {
        self.channels.insert(channel.address, channel.clone());
    }

    // TODO (raychu86) Clean up connections if peers are disconnected
}
