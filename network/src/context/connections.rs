use crate::message::Channel;

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
}
