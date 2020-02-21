use crate::message::Channel;
use snarkos_errors::network::ConnectError;
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

    /// Stores a new address => channel mapping. Returns the channel.
    pub fn store(&mut self, address: SocketAddr, channel: Channel) -> Arc<Channel> {
        let channel = Arc::new(channel);
        self.channels.insert(address, channel.clone());
        channel
    }

    /// Updates the address => channel mapping. Returns the channel.
    pub fn update(&mut self, old_address: SocketAddr, new_address: SocketAddr) -> Result<Arc<Channel>, ConnectError> {
        match self.channels.remove(&old_address) {
            Some(channel) => Ok(self.store(new_address, channel.update_address(new_address)?)),
            None => Err(ConnectError::AddressNotFound(old_address)),
        }
    }

    /// Connects to an address over tcp and stores a mapping for the new channel. Returns the channel.
    pub async fn connect_and_store(&mut self, address: SocketAddr) -> Result<Arc<Channel>, ConnectError> {
        let channel = Channel::connect(address).await?;

        Ok(self.store(address, channel.clone()))
    }
}
