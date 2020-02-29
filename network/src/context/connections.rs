use crate::message::Channel;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};

pub struct Connections {
    pub channels: HashMap<SocketAddr, Arc<Channel>>,
}

impl Connections {
    pub fn new() -> Self {
        Connections {
            channels: HashMap::<SocketAddr, Arc<Channel>>::new(),
        }
    }

    /// Returns the mapped channel if any
    pub fn get(&self, address: &SocketAddr) -> Option<Arc<Channel>> {
        self.channels.get(address).cloned()
    }

    pub fn store_channel(&mut self, channel: &Arc<Channel>) {
        self.channels.insert(channel.address, channel.clone());
    }

    /// Stores a new address => channel mapping. Returns the channel.
    pub fn store(&mut self, address: SocketAddr, channel: Channel) -> Arc<Channel> {
        let channel = Arc::new(channel);
        self.channels.insert(address, channel.clone());
        channel
    }
}
