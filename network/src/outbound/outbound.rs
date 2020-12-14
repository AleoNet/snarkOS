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

use crate::{outbound::Request, NetworkError};

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
    task,
    task::JoinHandle,
};

/// The TCP stream for sending outbound requests to a single remote address.
pub(super) type Channel = Arc<Mutex<TcpStream>>;

/// The map of remote addresses to their active write channels.
type Channels = HashMap<SocketAddr, Channel>;

/// The set of requests for a single remote address.
type Requests = HashSet<Request>;

/// The map of remote addresses to their pending requests.
type Pending = HashMap<SocketAddr, Requests>;

/// The map of remote addresses to their successful requests.
type Success = HashMap<SocketAddr, Requests>;

/// The map of remote addresses to their failed requests.
type Failure = HashMap<SocketAddr, Requests>;

/// A core data structure for handling outbound network traffic.
#[derive(Debug, Clone, Default)]
pub struct Outbound {
    /// The map of remote addresses to their active write channels.
    channels: Arc<RwLock<Channels>>,
    /// The map of remote addresses to their pending requests.
    pending: Arc<RwLock<Pending>>,
    /// The map of remote addresses to their successful requests.
    success: Arc<RwLock<Success>>,
    /// The map of remote addresses to their failed requests.
    failure: Arc<RwLock<Failure>>,
    /// The monotonic counter for the number of send requests the handler processes.
    send_pending_count: Arc<AtomicU64>,
    /// The monotonic counter for the number of send requests that succeeded.
    send_success_count: Arc<AtomicU64>,
    /// The monotonic counter for the number of send requests that failed.
    send_failure_count: Arc<AtomicU64>,
}

impl Outbound {
    ///
    /// Returns `true` if the given request is a pending request.
    ///
    #[inline]
    pub async fn is_pending(&self, request: &Request) -> bool {
        // Acquire the pending read lock.
        let pending = self.pending.read().await;
        // Fetch the pending requests of the given receiver.
        match pending.get(&request.receiver()) {
            // Check if the set of pending requests contains the given request.
            Some(requests) => requests.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Returns `true` if the given request was a successful request.
    ///
    #[inline]
    pub async fn is_success(&self, request: &Request) -> bool {
        // Acquire the success read lock.
        let success = self.success.read().await;
        // Fetch the successful requests of the given receiver.
        match success.get(&request.receiver()) {
            // Check if the set of successful requests contains the given request.
            Some(requests) => requests.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Returns `true` if the given request was a failed request.
    ///
    #[inline]
    pub async fn is_failure(&self, request: &Request) -> bool {
        // Acquire the failure read lock.
        let failure = self.failure.read().await;
        // Fetch the failed requests of the given receiver.
        match failure.get(&request.receiver()) {
            // Check if the set of failed requests contains the given request.
            Some(requests) => requests.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Broadcasts the given request.
    ///
    /// Creates or fetches an existing channel with the remote address,
    /// and attempts to send the given request to them.
    ///
    #[inline]
    pub async fn broadcast(&self, request: &Request) -> JoinHandle<()> {
        let outbound = self.clone();
        let request = request.clone();
        // Spawn a thread to send the request.
        task::spawn(async move {
            // Wait for authorization.
            outbound.authorize(&request).await;
            // Send the request.
            outbound.send(&request).await;
        })
    }
}

impl Outbound {
    ///
    /// Adds a new requests map for the given remote address to each state map,
    /// if it does not exist.
    ///
    #[inline]
    async fn initialize_state(&self, remote_address: SocketAddr) {
        debug!("Initializing Outbound state for {}", remote_address);
        self.pending.write().await.insert(remote_address, Default::default());
        self.success.write().await.insert(remote_address, Default::default());
        self.failure.write().await.insert(remote_address, Default::default());
    }

    ///
    /// Establishes an outbound channel to the given remote address, if it does not exist.
    ///
    #[inline]
    async fn outbound_channel(&self, remote_address: SocketAddr) -> Result<Channel, NetworkError> {
        let channel_exists = self.channels.read().await.contains_key(&remote_address);
        if !channel_exists {
            trace!("Establishing an outbound channel to {}", remote_address);
            let channel = TcpStream::connect(remote_address).await?;
            self.channels
                .write()
                .await
                .insert(remote_address, Arc::new(Mutex::new(channel)));

            self.initialize_state(remote_address).await;
        }
        Ok(self
            .channels
            .read()
            .await
            .get(&remote_address)
            .ok_or(NetworkError::OutboundChannelMissing)?
            .clone())
    }

    ///
    /// Authorizes the given request for broadcast to the corresponding outbound channel.
    ///
    #[inline]
    async fn authorize(&self, request: &Request) {
        // Acquire the pending requests write lock.
        let mut pending = self.pending.write().await;

        // Store the request to the pending requests.
        match pending.get_mut(&request.receiver()) {
            Some(requests) => {
                requests.insert(request.clone());

                // Increment the request counter.
                self.send_pending_count.fetch_add(1, Ordering::SeqCst);
            }
            None => warn!(
                "Failed to authorize `{}` request to {}",
                request.name(),
                request.receiver()
            ),
        };
    }

    #[inline]
    async fn send(&self, request: &Request) {
        // Fetch the outbound channel.
        let channel = match self.outbound_channel(request.receiver()).await {
            Ok(channel) => channel,
            Err(error) => {
                self.failure(&request, error).await;
                return;
            }
        };

        // Write the request to the outbound channel.
        match request.write_to_channel(&channel).await {
            Ok(_) => self.success(&request).await,
            Err(error) => self.failure(&request, error).await,
        };

        // TODO (howardwu): Add logic to determine whether to proceed with a disconnect.
        // // Disconnect from the peer if the version request fails to send.
        // if let Err(_) = channel.write(&version).await {
        //     self.disconnect_from_peer(&remote_address).await?;
        // }
    }

    #[inline]
    async fn success(&self, request: &Request) {
        // Acquire the pending requests write lock.
        let mut pending = self.pending.write().await;

        // Remove the request from the pending requests.
        if let Some(requests) = pending.get_mut(&request.receiver()) {
            requests.remove(&request);
        };

        // Acquire the success requests write lock.
        let mut success = self.success.write().await;

        // Store the request in the successful requests.
        if let Some(requests) = success.get_mut(&request.receiver()) {
            requests.insert(request.clone());

            // Increment the success counter.
            self.send_success_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[inline]
    async fn failure<E: Into<anyhow::Error> + Display>(&self, request: &Request, error: E) {
        warn!("Failed to send a {} to {}", request.name(), request.receiver());

        // Acquire the pending requests write lock.
        let mut pending = self.pending.write().await;

        // Remove the request from the pending requests.
        if let Some(requests) = pending.get_mut(&request.receiver()) {
            requests.remove(&request);
        };

        // Acquire the failed requests write lock.
        let mut failure = self.failure.write().await;

        // Store the request in the failed requests.
        if let Some(requests) = failure.get_mut(&request.receiver()) {
            requests.insert(request.clone());

            // Increment the failure counter.
            self.send_failure_count.fetch_add(1, Ordering::SeqCst);
        }

        trace!("{}", error);
    }
}

#[cfg(test)]
mod tests {
    use crate::{external::GetPeers, outbound::*};
    use snarkos_testing::network::{random_bound_address, TcpServer};

    use serial_test::serial;
    use std::{net::SocketAddr, sync::Arc, time::Duration};
    use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex, time::sleep};

    ///
    /// Returns a `Request` for testing.
    ///
    #[inline]
    fn request(remote_address: SocketAddr) -> Request {
        Request::GetPeers(remote_address, GetPeers)
    }

    ///
    /// Creates a new `TcpServer` and rejects requests if the given reject boolean is set to `true`.
    ///
    #[inline]
    async fn test_server_with_behavior(should_reject: bool) -> anyhow::Result<TcpServer> {
        let mut server = TcpServer::new().await;
        server.listen(should_reject).await.unwrap();

        Ok(server)
    }

    ///
    /// Creates a new `TcpServer`.
    ///
    #[inline]
    pub async fn test_server() -> anyhow::Result<TcpServer> {
        test_server_with_behavior(false).await
    }

    ///
    /// Creates a new `TcpServer` that rejects all requests.
    ///
    #[inline]
    pub async fn test_server_that_rejects() -> anyhow::Result<TcpServer> {
        test_server_with_behavior(true).await
    }

    #[tokio::test]
    async fn test_is_pending() {
        let (remote_address, _listener) = random_bound_address().await;
        let request = request(remote_address);

        // Create a new instance.
        let outbound = Outbound::default();
        outbound.initialize_state(remote_address).await;

        assert!(!outbound.is_pending(&request).await);
        assert!(!outbound.is_success(&request).await);
        assert!(!outbound.is_failure(&request).await);

        // Send the request to a non-existent server.
        let outbound_ = outbound.clone();
        let request_ = request.clone();
        tokio::task::spawn(async move {
            outbound_.broadcast(&request_).await;
        })
        .await
        .unwrap();

        // Check that the request failed.
        assert!(outbound.is_pending(&request).await);
        assert!(!outbound.is_success(&request).await);
        assert!(!outbound.is_failure(&request).await);
    }

    #[tokio::test]
    async fn test_is_success() {
        // Create a test server.
        let server = test_server().await.unwrap();
        let remote_address = server.address;

        let request = request(remote_address);

        // Create a new instance.
        let outbound = Outbound::default();
        outbound.initialize_state(remote_address).await;

        assert!(!outbound.is_pending(&request).await);
        assert!(!outbound.is_success(&request).await);
        assert!(!outbound.is_failure(&request).await);

        // Send the request to the server.
        outbound.broadcast(&request).await.await.unwrap();

        // Check that the request succeeded.
        assert!(!outbound.is_pending(&request).await);
        assert!(outbound.is_success(&request).await);
        assert!(!outbound.is_failure(&request).await);
    }

    #[tokio::test]
    async fn test_is_failure() {
        // Create a test server that refuses connections.
        let server = test_server_that_rejects().await.unwrap();
        let remote_address = server.address;

        let request = request(remote_address);

        // Create a new instance.
        let outbound = Outbound::default();
        outbound.initialize_state(remote_address).await;

        assert!(!outbound.is_pending(&request).await);
        assert!(!outbound.is_success(&request).await);
        assert!(!outbound.is_failure(&request).await);

        // Send the request to the server.
        outbound.broadcast(&request).await.await.unwrap();

        // Check that the request succeeded.
        assert!(!outbound.is_pending(&request).await);
        assert!(!outbound.is_success(&request).await);
        assert!(outbound.is_failure(&request).await);
    }

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
}
