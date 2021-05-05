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

use std::{collections::HashMap, net::SocketAddr, sync::atomic::Ordering};

use parking_lot::RwLock;
use tokio::sync::mpsc::{error::TrySendError, Receiver, Sender};

/// The map of remote addresses to their active write channels.
type Channels = HashMap<SocketAddr, Sender<Message>>;

/// A core data structure for handling outbound network traffic.
#[derive(Debug, Default)]
pub struct Outbound {
    /// The map of remote addresses to their active write channels.
    pub(crate) channels: RwLock<Channels>,
}

impl Outbound {
    ///
    /// Establishes an outbound channel to the given remote address, if it does not exist.
    ///
    #[inline]
    fn outbound_channel(&self, remote_address: SocketAddr) -> Result<Sender<Message>, NetworkError> {
        Ok(self
            .channels
            .read()
            .get(&remote_address)
            .ok_or(NetworkError::OutboundChannelMissing)?
            .clone())
    }
}

impl<S: Storage + Send + Sync + 'static> Node<S> {
    ///
    /// Sends the given request to the address associated with it.
    ///
    /// Fetches an existing channel with the remote address,
    /// and attempts to send the given request to it.
    ///
    #[inline]
    pub async fn send_request(&self, request: Message) {
        let target_addr = request.receiver();
        // Fetch the outbound channel.
        match self.outbound.outbound_channel(target_addr) {
            Ok(channel) => match channel.try_send(request) {
                Ok(()) => {}
                Err(TrySendError::Full(request)) => {
                    warn!(
                        "Couldn't send a {} to {}: the send channel is full",
                        request, target_addr
                    );
                    self.stats.outbound.all_failures.fetch_add(1, Ordering::Relaxed);
                }
                Err(TrySendError::Closed(request)) => {
                    error!(
                        "Couldn't send a {} to {}: the send channel is closed",
                        request, target_addr
                    );
                    self.stats.outbound.all_failures.fetch_add(1, Ordering::Relaxed);
                }
            },
            Err(_) => {
                warn!("Failed to send a {}: peer is disconnected", request);
                self.stats.outbound.all_failures.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub async fn send_ping(&self, remote_address: SocketAddr) {
        // Consider peering tests that don't use the sync layer.
        let current_block_height = if let Some(ref sync) = self.sync() {
            sync.current_block_height()
        } else {
            0
        };

        self.peer_book.sending_ping(remote_address);

        self.send_request(Message::new(
            Direction::Outbound(remote_address),
            Payload::Ping(current_block_height),
        ))
        .await;
    }

    /// This method handles new outbound messages to a single connected node.
    pub async fn listen_for_outbound_messages(&self, mut receiver: Receiver<Message>, writer: &mut ConnWriter) {
        loop {
            // Read the next message queued to be sent.
            if let Some(message) = receiver.recv().await {
                match writer.write_message(&message.payload).await {
                    Ok(_) => {
                        self.stats.outbound.all_successes.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(error) => {
                        warn!("Failed to send a {}: {}", message, error);
                        self.stats.outbound.all_failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
    }
}
