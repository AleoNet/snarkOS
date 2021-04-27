// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{ConnWriter, Direction, Message, NetworkError, Node, Payload};

use snarkvm_objects::Storage;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use parking_lot::RwLock;

/// The map of remote addresses to their active write channels.
type Channels = HashMap<SocketAddr, Arc<ConnWriter>>;

/// A core data structure for handling outbound network traffic.
#[derive(Debug)]
pub struct Outbound {
    /// The map of remote addresses to their active write channels.
    pub(crate) channels: Arc<RwLock<Channels>>,
    /// The monotonic counter for the number of send requests that succeeded.
    send_success_count: AtomicU64,
    /// The monotonic counter for the number of send requests that failed.
    send_failure_count: AtomicU64,
}

impl Outbound {
    pub fn new(channels: Arc<RwLock<Channels>>) -> Self {
        Self {
            channels,
            send_success_count: Default::default(),
            send_failure_count: Default::default(),
        }
    }

    ///
    /// Sends the given request to the address associated with it.
    ///
    /// Creates or fetches an existing channel with the remote address,
    /// and attempts to send the given request to them.
    ///
    #[inline]
    pub async fn send_request(&self, request: Message) {
        self.send(&request).await
    }

    ///
    /// Establishes an outbound channel to the given remote address, if it does not exist.
    ///
    #[inline]
    fn outbound_channel(&self, remote_address: SocketAddr) -> Result<Arc<ConnWriter>, NetworkError> {
        Ok(self
            .channels
            .read()
            .get(&remote_address)
            .ok_or(NetworkError::OutboundChannelMissing)?
            .clone())
    }

    async fn send(&self, request: &Message) {
        let target_addr = request.receiver();
        // Fetch the outbound channel.
        let channel = match self.outbound_channel(target_addr) {
            Ok(channel) => channel,
            Err(_) => {
                warn!("Failed to send a {}: peer is disconnected", request);
                return;
            }
        };

        // Write the request to the outbound channel.
        match channel.write_message(&request.payload).await {
            Ok(_) => {
                self.send_success_count.fetch_add(1, Ordering::SeqCst);
            }
            Err(error) => {
                warn!("Failed to send a {}: {}", request, error);
                self.send_failure_count.fetch_add(1, Ordering::SeqCst);
            }
        }
    }
}

impl<S: Storage + Send + Sync + 'static> Node<S> {
    pub async fn send_ping(&self, remote_address: SocketAddr) {
        // Consider peering tests that don't use the sync layer.
        let current_block_height = if let Some(ref sync) = self.sync() {
            sync.current_block_height()
        } else {
            0
        };

        self.peer_book.sending_ping(remote_address);

        self.outbound
            .send_request(Message::new(
                Direction::Outbound(remote_address),
                Payload::Ping(current_block_height),
            ))
            .await;
    }
}
