// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    helpers::{EventCodec, GatewayReceiver, Resolver},
    Event,
    Shared,
    CONTEXT,
    MAX_COMMITTEE_SIZE,
    MEMORY_POOL_PORT,
};
use snarkos_node_messages::DisconnectReason;
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    Config,
    Connection,
    ConnectionSide,
    Tcp,
    P2P,
};
use snarkvm::console::prelude::*;

use parking_lot::{Mutex, RwLock};
use std::{collections::HashSet, io, net::SocketAddr, sync::Arc};
use tokio::{sync::oneshot, task::JoinHandle};

#[derive(Clone)]
pub struct Gateway<N: Network> {
    /// The shared state.
    shared: Arc<Shared<N>>,
    /// The TCP stack.
    tcp: Tcp,
    /// The resolver.
    resolver: Arc<Resolver>,
    /// The map of connected peer IPs to their peer handlers.
    connected_peers: Arc<RwLock<HashSet<SocketAddr>>>,
    /// The set of handshaking peers. While `Tcp` already recognizes the connecting IP addresses
    /// and prevents duplicate outbound connection attempts to the same IP address, it is unable to
    /// prevent simultaneous "two-way" connections between two peers (i.e. both nodes simultaneously
    /// attempt to connect to each other). This set is used to prevent this from happening.
    connecting_peers: Arc<Mutex<HashSet<SocketAddr>>>,
}

impl<N: Network> Gateway<N> {
    /// Initializes a new gateway.
    pub fn new(shared: Arc<Shared<N>>) -> Result<Self> {
        // Initialize the worker IP.
        let worker_ip = SocketAddr::from_str(&format!("0.0.0.0:{MEMORY_POOL_PORT}"))?;
        // Initialize the TCP stack.
        let tcp = Tcp::new(Config::new(worker_ip, MAX_COMMITTEE_SIZE));
        // Return the gateway.
        Ok(Self {
            shared,
            tcp,
            resolver: Default::default(),
            connected_peers: Default::default(),
            connecting_peers: Default::default(),
        })
    }

    /// Run the gateway.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting the gateway for the memory pool");

        // Enable the TCP protocols.
        self.enable_handshake().await;
        self.enable_reading().await;
        self.enable_writing().await;
        self.enable_disconnect().await;
        self.enable_on_connect().await;
        // Enable the TCP listener. Note: This must be called after the above protocols.
        let _listening_addr = self.tcp.enable_listener().await.expect("Failed to enable the TCP listener");
        // // Initialize the heartbeat.
        // self.initialize_heartbeat();

        Ok(())
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> SocketAddr {
        self.tcp.listening_addr().expect("The TCP listener is not enabled")
    }

    /// Returns `true` if the given IP is this node.
    pub fn is_local_ip(&self, ip: &SocketAddr) -> bool {
        *ip == self.local_ip()
            || (ip.ip().is_unspecified() || ip.ip().is_loopback()) && ip.port() == self.local_ip().port()
    }

    /// Returns the listener IP address from the (ambiguous) peer address.
    pub fn resolve_to_listener(&self, peer_addr: &SocketAddr) -> Option<SocketAddr> {
        self.resolver.get_listener(peer_addr)
    }

    /// Returns the (ambiguous) peer address from the listener IP address.
    pub fn resolve_to_ambiguous(&self, peer_ip: &SocketAddr) -> Option<SocketAddr> {
        self.resolver.get_ambiguous(peer_ip)
    }

    /// Returns `true` if the node is connected to the given peer IP.
    pub fn is_connected(&self, ip: &SocketAddr) -> bool {
        self.connected_peers.read().contains(ip)
    }

    /// Returns the maximum number of connected peers.
    pub fn max_connected_peers(&self) -> usize {
        self.tcp.config().max_connections as usize
    }

    /// Returns the number of connected peers.
    pub fn number_of_connected_peers(&self) -> usize {
        self.connected_peers.read().len()
    }

    /// Returns the list of connected peers.
    pub fn connected_peers(&self) -> &RwLock<HashSet<SocketAddr>> {
        &self.connected_peers
    }

    /// Ensure the peer is allowed to connect.
    fn ensure_peer_is_allowed(&self, peer_ip: SocketAddr) -> Result<()> {
        // Ensure the peer IP is not this node.
        if self.is_local_ip(&peer_ip) {
            bail!("{CONTEXT} Dropping connection request from '{peer_ip}' (attempted to self-connect)")
        }
        // Ensure the node is not already connecting to this peer.
        if !self.connecting_peers.lock().insert(peer_ip) {
            bail!("{CONTEXT} Dropping connection request from '{peer_ip}' (already shaking hands as the initiator)")
        }
        // Ensure the node is not already connected to this peer.
        if self.is_connected(&peer_ip) {
            bail!("{CONTEXT} Dropping connection request from '{peer_ip}' (already connected)")
        }
        // // Ensure the peer is not restricted.
        // if self.is_restricted(&peer_ip) {
        //     bail!("Dropping connection request from '{peer_ip}' (restricted)")
        // }
        // // Ensure the peer is not spamming connection attempts.
        // if !peer_ip.ip().is_loopback() {
        //     // Add this connection attempt and retrieve the number of attempts.
        //     let num_attempts = self.cache.insert_inbound_connection(peer_ip.ip(), Self::RADIO_SILENCE_IN_SECS as i64);
        //     // Ensure the connecting peer has not surpassed the connection attempt limit.
        //     if num_attempts > Self::MAXIMUM_CONNECTION_FAILURES {
        //         // Restrict the peer.
        //         self.insert_restricted_peer(peer_ip);
        //         bail!("Dropping connection request from '{peer_ip}' (tried {num_attempts} times)")
        //     }
        // }
        Ok(())
    }

    /// Inserts the given peer into the connected peers.
    pub fn insert_connected_peer(&self, peer_ip: SocketAddr, peer_addr: SocketAddr) {
        // Adds a bidirectional map between the listener address and (ambiguous) peer address.
        self.resolver.insert_peer(peer_ip, peer_addr);
        // Add an entry for this peer in the connected peers.
        self.connected_peers.write().insert(peer_ip);
        // // Remove this peer from the candidate peers, if it exists.
        // self.candidate_peers.write().remove(&peer_ip);
        // // Remove this peer from the restricted peers, if it exists.
        // self.restricted_peers.write().remove(&peer_ip);
    }

    /// Removes the connected peer and adds them to the candidate peers.
    pub fn remove_connected_peer(&self, peer_ip: SocketAddr) {
        // Removes the bidirectional map between the listener address and (ambiguous) peer address.
        self.resolver.remove_peer(&peer_ip);
        // // Removes the peer from the sync pool.
        // self.sync.remove_peer(&peer_ip);
        // Remove this peer from the connected peers, if it exists.
        self.connected_peers.write().remove(&peer_ip);
        // // Add the peer to the candidate peers.
        // self.candidate_peers.write().insert(peer_ip);
    }

    /// Sends the given event to specified peer.
    ///
    /// This function returns as soon as the event is queued to be sent,
    /// without waiting for the actual delivery; instead, the caller is provided with a [`oneshot::Receiver`]
    /// which can be used to determine when and whether the event has been delivered.
    fn send(&self, peer_ip: SocketAddr, event: Event<N>) -> Option<oneshot::Receiver<io::Result<()>>> {
        // // Determine whether to send the event.
        // if !self.can_send(peer_ip, &event) {
        //     return None;
        // }
        // Resolve the listener IP to the (ambiguous) peer address.
        let peer_addr = match self.resolve_to_ambiguous(&peer_ip) {
            Some(peer_addr) => peer_addr,
            None => {
                warn!("Unable to resolve the listener IP address '{peer_ip}'");
                return None;
            }
        };
        // // If the event type is a block request, add it to the cache.
        // if let Event::BlockRequest(request) = event {
        //     self.router().cache.insert_outbound_block_request(peer_ip, request);
        // }
        // // If the event type is a puzzle request, increment the cache.
        // if matches!(event, Event::PuzzleRequest(_)) {
        //     self.router().cache.increment_outbound_puzzle_requests(peer_ip);
        // }
        // Retrieve the event name.
        let name = event.name();
        // Send the event to the peer.
        trace!("{CONTEXT} Sending '{name}' to '{peer_ip}'");
        let result = self.unicast(peer_addr, event);
        // If the event was unable to be sent, disconnect.
        if let Err(e) = &result {
            warn!("{CONTEXT} Failed to send '{name}' to '{peer_ip}': {e}");
            debug!("{CONTEXT} Disconnecting from '{peer_ip}' (unable to send)");
            self.disconnect(peer_ip);
        }
        result.ok()
    }

    /// Handles the inbound event from the peer.
    async fn inbound(&self, peer_addr: SocketAddr, event: Event<N>) -> Result<()> {
        // Retrieve the listener IP for the peer.
        let peer_ip = match self.resolve_to_listener(&peer_addr) {
            Some(peer_ip) => peer_ip,
            None => bail!("{CONTEXT} Unable to resolve the (ambiguous) peer address '{peer_addr}'"),
        };

        // // Drop the peer, if they have sent more than 1000 messages in the last 5 seconds.
        // let num_messages = self.router().cache.insert_inbound_message(peer_ip, 5);
        // if num_messages >= 1000 {
        //     bail!("Dropping '{peer_ip}' for spamming messages (num_messages = {num_messages})")
        // }

        trace!("{CONTEXT} Received '{}' from '{peer_ip}'", event.name());

        // This match statement handles the inbound event by deserializing the event,
        // checking the event is valid, and then calling the appropriate (trait) handler.
        match event {
            Event::Disconnect(disconnect) => {
                bail!("{CONTEXT} Disconnecting peer '{peer_ip}' for the following reason: {:?}", disconnect.reason)
            }
            Event::WorkerBatch(..) => {
                // Disconnect as the peer is not following the protocol.
                bail!("{CONTEXT} Peer '{peer_ip}' is not following the protocol")
            }
        }
    }

    /// Disconnects from the given peer IP, if the peer is connected.
    pub fn disconnect(&self, peer_ip: SocketAddr) -> JoinHandle<()> {
        let gateway = self.clone();
        tokio::spawn(async move {
            if let Some(peer_addr) = gateway.resolve_to_ambiguous(&peer_ip) {
                // Disconnect from this peer.
                let _disconnected = gateway.tcp.disconnect(peer_addr).await;
                debug_assert!(_disconnected);
            }
        })
    }
}

impl<N: Network> P2P for Gateway<N> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        &self.tcp
    }
}

#[async_trait]
impl<N: Network> Handshake for Gateway<N> {
    /// Performs the handshake protocol.
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Perform the handshake.
        let peer_addr = connection.addr();
        let conn_side = connection.side();
        // let stream = self.borrow_stream(&mut connection);

        // If this is an inbound connection, we log it, but don't know the listening address yet.
        // Otherwise, we can immediately register the listening address.
        let mut peer_ip = if conn_side == ConnectionSide::Initiator {
            debug!("{CONTEXT} Gateway received a connection request from '{peer_addr}'");
            None
        } else {
            debug!("{CONTEXT} Gateway is connecting to {peer_addr}...");
            Some(peer_addr)
        };

        // Perform the handshake; we pass on a mutable reference to peer_ip in case the process is broken at any point in time.
        if conn_side == ConnectionSide::Responder {
            // This value is immediately guaranteed to be present, so it can be unwrapped.
            let peer_ip = peer_ip.unwrap();

            // Add the peer to the gateway.
            self.insert_connected_peer(peer_ip, peer_addr);
        } else {
            // Obtain the peer's listening address.
            peer_ip = Some(SocketAddr::new(peer_addr.ip(), MEMORY_POOL_PORT));
            let peer_ip = peer_ip.unwrap();

            // Knowing the peer's listening address, ensure it is allowed to connect.
            if let Err(forbidden_error) = self.ensure_peer_is_allowed(peer_ip) {
                return Err(error(format!("{forbidden_error}")));
            }
        }

        // Remove the address from the collection of connecting peers (if the handshake got to the point where it's known).
        if let Some(peer_ip) = peer_ip {
            self.connecting_peers.lock().remove(&peer_ip);
            info!("{CONTEXT} Gateway is connected to '{peer_ip}'");
        }

        Ok(connection)
    }
}

#[async_trait]
impl<N: Network> OnConnect for Gateway<N> {
    async fn on_connect(&self, peer_addr: SocketAddr) {
        let _peer_ip = if let Some(ip) = self.resolve_to_listener(&peer_addr) {
            ip
        } else {
            return;
        };

        // // Retrieve the block locators.
        // let block_locators = match crate::helpers::get_block_locators(&self.ledger) {
        //     Ok(block_locators) => Some(block_locators),
        //     Err(e) => {
        //         error!("Failed to get block locators: {e}");
        //         return;
        //     }
        // };
        //
        // // Send the first `Ping` message to the peer.
        // self.send_ping(peer_ip, block_locators);
    }
}

#[async_trait]
impl<N: Network> Writing for Gateway<N> {
    type Codec = EventCodec<N>;
    type Message = Event<N>;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network> Reading for Gateway<N> {
    type Codec = EventCodec<N>;
    type Message = Event<N>;

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    /// Processes a message received from the network.
    async fn process_message(&self, peer_addr: SocketAddr, message: Self::Message) -> io::Result<()> {
        // Process the message. Disconnect if the peer violated the protocol.
        if let Err(error) = self.inbound(peer_addr, message).await {
            if let Some(peer_ip) = self.resolve_to_listener(&peer_addr) {
                warn!("Disconnecting from '{peer_ip}' - {error}");
                self.send(peer_ip, Event::Disconnect(DisconnectReason::ProtocolViolation.into()));
                // Disconnect from this peer.
                self.disconnect(peer_ip);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network> Disconnect for Gateway<N> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        if let Some(peer_ip) = self.resolve_to_listener(&peer_addr) {
            self.remove_connected_peer(peer_ip);
        }
    }
}
