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

use crate::{external::Channel, outbound::Request, NetworkError};

use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::sync::RwLock;

/// The map of remote addresses to their active write channels.
type Channels = HashMap<SocketAddr, Arc<Channel>>;

/// The set of requests for a single peer.
type Requests = Arc<RwLock<HashSet<Request>>>;

/// The map of remote addresses to their pending requests.
type Pending = HashMap<SocketAddr, Requests>;

/// The map of remote addresses to their successful requests.
type Success = HashMap<SocketAddr, Requests>;

/// The map of remote addresses to their failed requests.
type Failure = HashMap<SocketAddr, Requests>;

/// A core data structure for handling outbound network traffic.
#[derive(Debug, Clone)]
pub struct Outbound {
    /// The map of remote addresses to their active write channels.
    channels: Arc<RwLock<Channels>>,
    /// The map of remote addresses to their pending requests.
    pending: Arc<RwLock<Pending>>,
    /// The map of remote addresses to their successful requests.
    success: Arc<RwLock<Success>>,
    /// The map of remote addresses to their failed requests.
    failure: Arc<RwLock<Failure>>,
    /// The counter for the number of send requests the handler processes.
    send_request_count: Arc<AtomicU64>,
    /// The counter for the number of send requests that succeeded.
    send_success_count: Arc<AtomicU64>,
    /// The counter for the number of send requests that failed.
    send_failure_count: Arc<AtomicU64>,
}

impl Outbound {
    /// Creates a new instance of a `Outbound`.
    #[inline]
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(Pending::new())),
            success: Arc::new(RwLock::new(Success::new())),
            failure: Arc::new(RwLock::new(Failure::new())),
            send_request_count: Arc::new(AtomicU64::new(0)),
            send_success_count: Arc::new(AtomicU64::new(0)),
            send_failure_count: Arc::new(AtomicU64::new(0)),
        }
    }

    ///
    /// Returns `true` if the given request is a pending request.
    ///
    pub async fn is_pending(&self, request: &Request) -> bool {
        // Acquire the pending read lock.
        let pending = self.pending.read().await;
        // Fetch the pending requests of the given receiver.
        match pending.get(&request.receiver()) {
            // Check if the set of pending requests contains the given request.
            Some(requests) => requests.read().await.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Returns `true` if the given request was a successful request.
    ///
    pub async fn is_success(&self, request: &Request) -> bool {
        // Acquire the success read lock.
        let success = self.success.read().await;
        // Fetch the successful requests of the given receiver.
        match success.get(&request.receiver()) {
            // Check if the set of successful requests contains the given request.
            Some(requests) => requests.read().await.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Returns `true` if the given request was a failed request.
    ///
    pub async fn is_failure(&self, request: &Request) -> bool {
        // Acquire the failure read lock.
        let failure = self.failure.read().await;
        // Fetch the failed requests of the given receiver.
        match failure.get(&request.receiver()) {
            // Check if the set of failed requests contains the given request.
            Some(requests) => requests.read().await.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Broadcasts the given request.
    ///
    /// Broadcasts a handshake request with a given version message.
    ///
    /// Creates a new handshake with a remote address,
    /// and attempts to send a handshake request to them.
    ///
    /// Upon success, the handshake is stored in the manager.
    ///
    #[inline]
    pub async fn broadcast(&self, request: &Request) -> Result<(), NetworkError> {
        // Wait for authorization to send the request.
        let (channel, pending_requests) = match self.authorize(request).await {
            Ok((channel, pending_requests)) => (channel, pending_requests),
            Err(error) => {
                error!(
                    "Unauthorized to send `{}` request to {}\n{}",
                    request.name(),
                    request.receiver(),
                    error
                );
                return Err(NetworkError::SendRequestUnauthorized);
            }
        };

        debug!("Sending request to {:?}", request.receiver());

        // Clone these variables for use in the thread.
        let request = request.clone();
        let send_success_count = self.send_success_count.clone();
        let send_failure_count = self.send_failure_count.clone();

        // Spawn a thread to handle sending the request.
        tokio::task::spawn(async move {
            // Fetch the request receiver.
            let receiver = request.receiver();

            trace!("Sending request to {:?}", receiver);

            // TODO (howardwu): Abstract this with a trait object or generic.
            let result = match &request {
                Request::Block(_, payload) => channel.write(payload).await,
                Request::GetPeers(_, payload) => channel.write(payload).await,
                Request::Peers(_, payload) => channel.write(payload).await,
                Request::Transaction(_, payload) => channel.write(payload).await,
                Request::Verack(payload) => channel.write(payload).await,
                Request::Version(payload) => channel.write(payload).await,
            };

            // Write the version message to the channel.
            match result {
                Ok(_) => {
                    // Increment the success counter.
                    send_success_count.fetch_add(1, Ordering::SeqCst);
                    trace!("Sent request to {:?}", receiver);
                }
                Err(error) => {
                    // Increment the failed counter.
                    send_failure_count.fetch_add(1, Ordering::SeqCst);
                    error!("Failed to send request to {:?}", receiver);

                    // TODO (howardwu): Add logic to determine whether to proceed with a disconnect.
                    // // Disconnect from the peer if the version request fails to send.
                    // if let Err(_) = channel.write(&version).await {
                    //     self.disconnect_from_peer(&remote_address).await?;
                    // }
                }
            }

            // Acquire the pending requests write lock.
            let mut writer = pending_requests.write().await;
            // Remove the request from the pending requests.
            writer.remove(&request);
        });

        Ok(())
    }

    // TODO (howardwu): Implement getters to all counters.
    // /// Returns the number of requests the manager has processed.
    // #[inline]
    // pub async fn get_request_count(&self) -> u64 {
    //     let counter = self.send_request_count.clone();
    //     counter.clone().into_inner()
    // }

    async fn authorize(&self, request: &Request) -> Result<(Arc<Channel>, Requests), NetworkError> {
        // Fetch the request receiver.
        let receiver = request.receiver();
        trace!("Authorizing `{}` request to {}", request.name(), receiver);

        // Acquire the channels write lock.
        let mut channels = self.channels.write().await;
        // Acquire the pending write lock.
        let mut pending = self.pending.write().await;

        // Fetch or initialize the channel for broadcasting the request.
        let channel = if let Some(channel) = channels.get(&receiver) {
            // Case 1 - The channel exists, retrieves the channel.
            trace!("Using the existing channel with {}", receiver);
            channel.clone()
        } else {
            // Case 2 - The channel does not exist, creates and returns a new channel.
            trace!("Creating a new channel with {}", receiver);
            if let Ok(channel) = Channel::new_writer(receiver.clone()).await {
                trace!("Created a new channel with {}", receiver);
                Arc::new(channel)
            } else {
                error!("Failed to create a new channel with {}", receiver);
                return Err(NetworkError::OutboundFailedToCreateChannel);
            }
        };

        // Fetch or initialize the pending requests for the request receiver.
        let pending_requests = if let Some(requests) = pending.get(&receiver) {
            // Case 1 - The receiver exists, retrieves the requests.
            trace!("Using the existing instance of pending requests");
            requests.clone()
        } else {
            // Case 2 - The receiver does not exist, initializes requests and stores it.
            trace!("Creating a new instance of pending requests");
            // Creates a new instance of `Requests` and stores it.
            Arc::new(RwLock::new(HashSet::new()))
        };

        // Acquire the pending requests write lock.
        let mut pending_inner = pending_requests.write().await;

        // Store the channel in the channel map.
        channels.insert(receiver, channel.clone());
        // Store the pending requests in the pending map.
        pending.insert(receiver, pending_requests.clone());
        // Store the request to the pending requests.
        pending_inner.insert(request.clone());

        // Increment the request counter.
        self.send_request_count.fetch_add(1, Ordering::SeqCst);

        trace!("Authorized `{}` request to {}", request.name(), receiver);
        Ok((channel, pending_requests.clone()))
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::external::{Channel, Message, Verack, Version};
//     use snarkos_testing::network::random_socket_address;
//
//     use serial_test::serial;
//     use tokio::net::TcpListener;
//
//     #[tokio::test]
//     #[serial]
//     async fn test_send_and_receive_handshake() {
//         let local_address = random_socket_address();
//         let remote_address = random_socket_address();
//
//         // 1. Bind to remote address.
//         let mut remote_listener = TcpListener::bind(remote_address).await.unwrap();
//         let mut remote_manager = Outbound::new();
//
//         tokio::spawn(async move {
//             let mut local_listener = TcpListener::bind(local_address).await.unwrap();
//             let mut local_manager = Outbound::new();
//
//             // 2. Local node sends handshake request
//             let local_version = Version::new_with_rng(1u64, 0u32, local_address, remote_address);
//             local_manager.broadcast(&Request::Version(local_version)).await;
//
//             // 5. Check local node handshake state
//             let (reader, _) = local_listener.accept().await.unwrap();
//             let channel = Channel::new_reader(reader).unwrap();
//
//             // 6. Local node accepts handshake response
//             let (_name, bytes) = channel.read().await.unwrap();
//             let verack = Verack::deserialize(bytes).unwrap();
//             local_manager.accept_response(remote_address, verack).await;
//
//             // 7. Local node receives handshake request
//             let (_name, bytes) = channel.read().await.unwrap();
//             let remote_version = Version::deserialize(bytes).unwrap();
//
//             // 8. Local node sends handshake response
//             local_manager.receive_request(remote_version, remote_address).await;
//         });
//
//         // 3. Remote node accepts Local node connection
//         let (reader, _) = remote_listener.accept().await.unwrap();
//
//         // 4. Remote node sends handshake response, handshake request
//         let (handshake, _, _) = remote_manager
//             .receive_connection_request(1u64, 0u32, local_address, reader)
//             .await
//             .unwrap();
//
//         // 9. Local node accepts handshake response
//         let (_, bytes) = handshake.channel.read().await.unwrap();
//         let verack = Verack::deserialize(bytes).unwrap();
//         remote_manager.accept_response(local_address, verack).await;
//     }
// }
