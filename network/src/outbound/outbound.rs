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
    fmt::Display,
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use parking_lot::RwLock;
use tokio::{task, task::JoinHandle};

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
#[derive(Debug, Clone)]
pub struct Outbound {
    /// The map of remote addresses to their active write channels.
    pub(crate) channels: Arc<RwLock<Channels>>,
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
    pub fn new(channels: Arc<RwLock<Channels>>) -> Self {
        Self {
            channels,
            pending: Default::default(),
            success: Default::default(),
            failure: Default::default(),
            send_pending_count: Default::default(),
            send_success_count: Default::default(),
            send_failure_count: Default::default(),
        }
    }

    ///
    /// Returns `true` if the given request is a pending request.
    ///
    #[inline]
    pub fn is_pending(&self, request: &Request) -> bool {
        // Fetch the pending requests of the given receiver.
        match self.pending.read().get(&request.receiver()) {
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
    pub fn is_success(&self, request: &Request) -> bool {
        // Fetch the successful requests of the given receiver.
        match self.success.read().get(&request.receiver()) {
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
    pub fn is_failure(&self, request: &Request) -> bool {
        // Fetch the failed requests of the given receiver.
        match self.failure.read().get(&request.receiver()) {
            // Check if the set of failed requests contains the given request.
            Some(requests) => requests.contains(&request),
            // Return `false` as the receiver does not exist in this map.
            None => false,
        }
    }

    ///
    /// Sends the given request to the address associated with it.
    ///
    /// Creates or fetches an existing channel with the remote address,
    /// and attempts to send the given request to them.
    ///
    #[inline]
    pub async fn send_request(&self, request: &Request) -> JoinHandle<()> {
        let outbound = self.clone();
        let request = request.clone();

        tokio::spawn(async move {
            // Wait for authorization.
            outbound.authorize(&request).await;
            // Send the request.
            outbound.send(&request).await;
        })
    }

    ///
    /// Adds a new requests map for the given remote address to each state map,
    /// if it does not exist.
    ///
    #[inline]
    pub async fn initialize_state(&self, remote_address: SocketAddr) {
        debug!("Initializing Outbound state for {}", remote_address);
        self.pending.write().insert(remote_address, Default::default());
        self.success.write().insert(remote_address, Default::default());
        self.failure.write().insert(remote_address, Default::default());
    }

    ///
    /// Establishes an outbound channel to the given remote address, if it does not exist.
    ///
    #[inline]
    async fn outbound_channel(&self, remote_address: SocketAddr) -> Result<Channel, NetworkError> {
        Ok(self
            .channels
            .read()
            .get(&remote_address)
            .ok_or(NetworkError::OutboundChannelMissing)?
            .clone())
    }

    ///
    /// Authorizes the given request to be sent to the corresponding outbound channel.
    ///
    #[inline]
    async fn authorize(&self, request: &Request) {
        // Store the request to the pending requests.
        match self.pending.write().get_mut(&request.receiver()) {
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
    }

    #[inline]
    async fn success(&self, request: &Request) {
        // Remove the request from the pending requests.
        if let Some(requests) = self.pending.write().get_mut(&request.receiver()) {
            requests.remove(&request);
        };

        // Store the request in the successful requests.
        if let Some(requests) = self.success.write().get_mut(&request.receiver()) {
            requests.insert(request.clone());

            // Increment the success counter.
            self.send_success_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[inline]
    async fn failure<E: Into<anyhow::Error> + Display>(&self, request: &Request, error: E) {
        warn!(
            "Failed to send a {} to {}: {}",
            request.name(),
            request.receiver(),
            error
        );

        // Remove the request from the pending requests.
        if let Some(requests) = self.pending.write().get_mut(&request.receiver()) {
            requests.remove(&request);
        };

        // Store the request in the failed requests.
        if let Some(requests) = self.failure.write().get_mut(&request.receiver()) {
            requests.insert(request.clone());

            // Increment the failure counter.
            self.send_failure_count.fetch_add(1, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{external::GetPeers, outbound::*, Channel};
    use snarkos_testing::network::TcpServer;

    use std::net::SocketAddr;
    use tokio::net::TcpStream;

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
        let remote_address = "127.0.0.1:7777".parse::<SocketAddr>().unwrap();
        let request = request(remote_address);

        // Create a new instance.
        let outbound = Outbound::new(Default::default());
        outbound.initialize_state(remote_address).await;

        assert!(!outbound.is_pending(&request));
        assert!(!outbound.is_success(&request));
        assert!(!outbound.is_failure(&request));

        // Authorize the request only.
        outbound.authorize(&request).await;

        // Check that the request is only pending.
        assert!(outbound.is_pending(&request));
        assert!(!outbound.is_success(&request));
        assert!(!outbound.is_failure(&request));
    }

    #[tokio::test]
    async fn test_is_success() {
        // Create a test server.
        let server = test_server().await.unwrap();
        let stream = TcpStream::connect(server.address).await.unwrap();
        let remote_address = stream.peer_addr().unwrap();
        let request = request(remote_address);

        // Create a new instance.
        let outbound = Outbound::new(Default::default());
        let channel = Channel::new(remote_address, stream);
        outbound.channels.write().insert(remote_address, channel);
        outbound.initialize_state(remote_address).await;

        assert!(!outbound.is_pending(&request));
        assert!(!outbound.is_success(&request));
        assert!(!outbound.is_failure(&request));

        // Send the request to the server.
        outbound.send_request(&request).await.await.unwrap();

        // Check that the request succeeded.
        assert!(!outbound.is_pending(&request));
        assert!(outbound.is_success(&request));
        assert!(!outbound.is_failure(&request));
    }

    #[tokio::test]
    async fn test_is_failure() {
        // Create a test server that refuses connections.
        let server = test_server_that_rejects().await.unwrap();
        let remote_address = server.address;

        let request = request(remote_address);

        // Create a new instance.
        let outbound = Outbound::new(Default::default());
        outbound.initialize_state(remote_address).await;

        assert!(!outbound.is_pending(&request));
        assert!(!outbound.is_success(&request));
        assert!(!outbound.is_failure(&request));

        // Send the request to the server.
        outbound.send_request(&request).await.await.unwrap();

        // Check that the request succeeded.
        assert!(!outbound.is_pending(&request));
        assert!(!outbound.is_success(&request));
        assert!(outbound.is_failure(&request));
    }
}
