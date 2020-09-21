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

use crate::external::{message::message::Message, message_types::Version, Channel, Handshake, Verack};

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{net::TcpStream, sync::RwLock, task};

#[derive(Debug, Clone)]
pub struct RequestManager {
    /// The handshakes with connected peers
    handshakes: Arc<RwLock<HashMap<SocketAddr, Handshake>>>,
    /// A list of remote addresses currently sending a request.
    pending_addresses: Arc<RwLock<HashSet<SocketAddr>>>,
    /// A counter for the number of send requests the manager processes.
    send_request_count: Arc<AtomicU64>,
    /// A counter for the number of send requests that succeeded.
    send_success_count: Arc<AtomicU64>,
    /// A counter for the number of send requests that failed.
    send_failure_count: Arc<AtomicU64>,
}

impl RequestManager {
    /// Creates a new instance of a `ConnectionManager`.
    #[inline]
    pub fn new() -> Self {
        Self {
            handshakes: Arc::new(RwLock::new(HashMap::default())),
            pending_addresses: Arc::new(RwLock::new(HashSet::default())),
            send_request_count: Arc::new(AtomicU64::new(0)),
            send_success_count: Arc::new(AtomicU64::new(0)),
            send_failure_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Returns the list of remote addresses currently sending a request.
    #[inline]
    pub async fn get_pending_addresses(&self) -> Vec<SocketAddr> {
        // Acquire the pending addresses read lock.
        let pending_addresses = self.pending_addresses.read().await;
        pending_addresses.clone().into_iter().collect()
    }

    // TODO (howardwu): Implement getters to all counters.
    // /// Returns the number of requests the manager has processed.
    // #[inline]
    // pub async fn get_request_count(&self) -> u64 {
    //     let counter = self.send_request_count.clone();
    //     counter.clone().into_inner()
    // }

    /// Returns the nonce for a handshake with the given remote address, if it exists.
    #[inline]
    pub async fn get_handshake_nonce(&self, remote_address: &SocketAddr) -> Option<u64> {
        // Acquire the handshakes read lock.
        let handshakes = self.handshakes.read().await;
        match handshakes.get(remote_address) {
            Some(handshake) => Some(handshake.nonce),
            _ => None,
        }
    }

    /// Sends a connection request with a given version message.
    #[inline]
    pub async fn send_connection_request(&self, version: &Version) {
        self.send_handshake(version).await;
    }

    /// Receives a connection request with a given version message.
    #[inline]
    pub async fn receive_connection_request(
        &mut self,
        version: u64,
        block_height: u32,
        remote_address: SocketAddr,
        reader_stream: TcpStream,
    ) -> Option<(Handshake, SocketAddr, Option<Version>)> {
        self.receive_handshake(version, block_height, remote_address, reader_stream)
            .await
    }

    // TODO (howardwu): Review this again.
    /// Receives a handshake request from a connected peer.
    /// Updates the handshake channel address, if needed.
    /// Sends a handshake response back to the connected peer.
    pub async fn receive_request(&mut self, message: Version, remote_address: SocketAddr) -> bool {
        match self.handshakes.write().await.get_mut(&remote_address) {
            Some(handshake) => {
                handshake.update_address(remote_address);
                handshake.receive(message).await.is_ok()
            }
            None => false,
        }
    }

    // TODO (howardwu): Review this again.
    /// Accepts a handshake response from a connected peer.
    pub async fn accept_response(&mut self, remote_address: SocketAddr, message: Verack) -> bool {
        match self.handshakes.write().await.get_mut(&remote_address) {
            Some(handshake) => {
                debug!("New handshake with {:?}", remote_address);
                handshake.accept(message).await.is_ok()
            }
            None => false,
        }
    }

    ///
    /// Broadcasts a handshake request with a given version message.
    ///
    /// Creates a new handshake with a remote address,
    /// and attempts to send a handshake request to them.
    ///
    /// Upon success, the handshake is stored in the manager.
    ///
    #[inline]
    async fn send_handshake(&self, version: &Version) {
        // Increment the request counter.
        self.send_request_count.fetch_add(1, Ordering::SeqCst);

        // Clone an instance of version, handshakes, pending_addresses,
        // send_success_count, and send_failure_count for the tokio thread.
        let version = version.clone();
        let handshakes = self.handshakes.clone();
        let pending_addresses = self.pending_addresses.clone();
        let send_success_count = self.send_success_count.clone();
        let send_failure_count = self.send_failure_count.clone();

        // Spawn a new thread to make this a non-blocking operation.
        task::spawn(async move {
            // Get the remote address for logging.
            let remote_address = version.address_receiver;
            info!("Attempting connection to {:?}", remote_address);

            // Acquire the pending addresses write lock.
            let mut pending_peers = pending_addresses.write().await;
            // Add the remote address to the pending addresses.
            pending_peers.insert(remote_address);
            // Drop the pending addresses write lock.
            drop(pending_peers);

            // Acquire the handshake and pending addresses write locks.
            let mut handshakes = handshakes.write().await;
            // Attempt a handshake with the remote address.
            debug!("Requesting handshake with {:?}", remote_address);
            match Handshake::send_new(&version).await {
                Ok(handshake) => {
                    // Store the handshake.
                    handshakes.insert(remote_address, handshake);
                    // Increment the success counter.
                    send_success_count.fetch_add(1, Ordering::SeqCst);
                    debug!("Sent handshake to {:?}", remote_address);
                }
                _ => {
                    // Increment the failed counter.
                    send_failure_count.fetch_add(1, Ordering::SeqCst);
                    info!("Unsuccessful connection with {:?}", remote_address);
                }
            };
            // Drop the handshake write lock.
            drop(handshakes);

            // Acquire the pending addresses write lock.
            let mut pending_peers = pending_addresses.write().await;
            // Remove the remote address from the pending addresses.
            pending_peers.remove(&remote_address);
            // Drop the pending addresses write lock.
            drop(pending_peers);
        });
    }

    ///
    /// Listens for the first message request from a remote peer.
    ///
    /// If the message is a Version:
    ///
    ///     1. Create a new handshake.
    ///     2. Send a handshake response.
    ///     3. If the response is sent successfully, store the handshake.
    ///     4. Return the handshake, your address as seen by sender, and the version message.
    ///
    /// If the message is a Verack:
    ///
    ///     1. Get the existing handshake.
    ///     2. Mark the handshake as accepted.
    ///     3. Send a request for peers.
    ///     4. Return the accepted handshake and your address as seen by sender.
    ///
    pub async fn receive_handshake(
        &mut self,
        version: u64,
        height: u32,
        peer_address: SocketAddr,
        reader: TcpStream,
    ) -> Option<(Handshake, SocketAddr, Option<Version>)> {
        // Read the first message or return `None`.
        let channel = Channel::new_read_only(reader);
        // Parse the inbound message into the message name and message bytes.
        let (channel, (message_name, message_bytes)) = match channel {
            // Read the next message from the channel.
            // Note this is a blocking operation.
            Ok(channel) => match channel.read().await {
                Ok(inbound_message) => (channel, inbound_message),
                _ => return None,
            },
            _ => return None,
        };

        // Handles a version message request.
        // Create and store a new handshake in the manager.
        if message_name == Version::name() {
            // Deserialize the message bytes into a version message.
            let remote_version = match Version::deserialize(message_bytes) {
                Ok(remote_version) => remote_version,
                _ => return None,
            };
            let local_address = remote_version.address_receiver;
            // Create the remote address from the given peer address, and specified port from the version message.
            let remote_address = SocketAddr::new(peer_address.ip(), remote_version.address_sender.port());
            // Create the local version message.
            let local_version = Version::new(version, height, remote_address, local_address);
            // Process the new version message and send a response to the remote peer.
            let handshake = match Handshake::receive_new(channel, &local_version, &remote_version).await {
                Ok(handshake) => handshake,
                _ => return None,
            };
            debug!("Received handshake from {:?}", remote_address);
            // Acquire the handshake write lock.
            let mut handshakes = self.handshakes.write().await;
            // Store the new handshake.
            handshakes.insert(remote_address, handshake.clone());
            // Drop the handshakes write lock.
            drop(handshakes);
            return Some((handshake, local_address, Some(local_version)));
        }

        // Handles a verack message request.
        // Establish the channel with the remote peer.
        if message_name == Verack::name() {
            // Deserialize the message bytes into a verack message.
            let verack = match Verack::deserialize(message_bytes) {
                Ok(verack) => verack,
                _ => return None,
            };
            let local_address = verack.address_receiver;
            // TODO (howardwu): Check whether this remote address needs to
            //   be derive the same way as the version message case above
            //  (using a peer_address.ip() and address_sender.port()).
            let remote_address = verack.address_sender;
            // Acquire the handshake write lock.
            let mut handshakes = self.handshakes.write().await;
            // Accept the handshake with the remote address.
            let result = match handshakes.get_mut(&remote_address) {
                Some(handshake) => match handshake.accept(verack).await {
                    Ok(()) => {
                        handshake.update_reader(channel);
                        info!("New handshake with {:?}", remote_address);
                        Some((handshake.clone(), local_address, None))
                    }
                    _ => None,
                },
                _ => None,
            };
            // Drop the handshakes write lock.
            drop(handshakes);
            return result;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::{Channel, HandshakeStatus, Message};
    use snarkos_testing::network::random_socket_address;

    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_send_and_receive_handshake() {
        let local_address = random_socket_address();
        let remote_address = random_socket_address();

        // 1. Bind to remote address.
        let mut remote_listener = TcpListener::bind(remote_address).await.unwrap();
        let mut remote_manager = RequestManager::new();

        tokio::spawn(async move {
            let mut local_listener = TcpListener::bind(local_address).await.unwrap();
            let mut local_manager = RequestManager::new();

            // 2. Local node sends handshake request
            let local_version = Version::new(1u64, 0u32, remote_address, local_address);
            local_manager.send_connection_request(&local_version).await.unwrap();

            // 5. Check local node handshake state
            let (reader, _) = local_listener.accept().await.unwrap();
            let channel = Channel::new_read_only(reader).unwrap();
            assert_eq!(HandshakeStatus::Waiting, handshake.get_state(remote_address).unwrap());

            // 6. Local node accepts handshake response
            let (_name, bytes) = channel.read().await.unwrap();
            let verack = Verack::deserialize(bytes).unwrap();

            local_manager.accept_response(remote_address, verack).await.unwrap();
            assert_eq!(HandshakeStatus::Accepted, handshake.get_state(remote_address).unwrap());

            // 7. Local node receives handshake request
            let (_name, bytes) = channel.read().await.unwrap();
            let remote_version = Version::deserialize(bytes).unwrap();

            // 8. Local node sends handshake response
            local_manager
                .receive_request(remote_version, remote_address)
                .await
                .unwrap();
        });

        // 3. Remote node accepts Local node connection
        let (reader, _) = remote_listener.accept().await.unwrap();

        // 4. Remote node sends handshake response, handshake request
        let (handshake, _, _) = remote_manager
            .receive_any(1u64, 0u32, local_address, reader)
            .await
            .unwrap();
        assert_eq!(HandshakeStatus::Waiting, handshakes.get_state(local_address).unwrap());

        // 9. Local node accepts handshake response
        let (_, bytes) = handshake.channel.read().await.unwrap();
        let verack = Verack::deserialize(bytes).unwrap();

        handshakes.accept_response(local_address, verack).await.unwrap();

        assert_eq!(HandshakeStatus::Accepted, handshakes.get_state(local_address).unwrap())
    }
}
