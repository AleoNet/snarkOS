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
    event::{
        CertificateRequest,
        CertificateResponse,
        ChallengeRequest,
        ChallengeResponse,
        DisconnectReason,
        Event,
        EventTrait,
        TransmissionRequest,
        TransmissionResponse,
    },
    helpers::{assign_to_worker, Cache, EventCodec, PrimarySender, Resolver, Storage, WorkerSender},
    CONTEXT,
    MAX_BATCH_DELAY,
    MAX_GC_ROUNDS,
    MAX_TRANSMISSIONS_PER_BATCH,
    MEMORY_POOL_PORT,
};
use snarkos_account::Account;
use snarkos_node_narwhal_committee::MAX_COMMITTEE_SIZE;
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    Config,
    Connection,
    ConnectionSide,
    Tcp,
    P2P,
};
use snarkvm::{console::prelude::*, ledger::narwhal::Data, prelude::Address};

use futures::SinkExt;
use indexmap::{IndexMap, IndexSet};
use parking_lot::{Mutex, RwLock};
use std::{future::Future, io, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::TcpStream,
    sync::{oneshot, OnceCell},
    task::{self, JoinHandle},
};
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

/// The maximum interval of events to cache.
const CACHE_EVENTS_INTERVAL: i64 = (MAX_BATCH_DELAY / 1000) as i64; // seconds
/// The maximum interval of requests to cache.
const CACHE_REQUESTS_INTERVAL: i64 = (MAX_BATCH_DELAY / 1000) as i64; // seconds

/// The maximum number of events to cache.
const CACHE_EVENTS: usize = CACHE_TRANSMISSIONS;
/// The maximum number of certificate requests to cache.
const CACHE_CERTIFICATES: usize = 2 * MAX_GC_ROUNDS as usize * MAX_COMMITTEE_SIZE as usize;
/// The maximum number of transmission requests to cache.
const CACHE_TRANSMISSIONS: usize = CACHE_CERTIFICATES * MAX_TRANSMISSIONS_PER_BATCH;
/// The maximum number of duplicates for any particular request.
const CACHE_MAX_DUPLICATES: usize = MAX_COMMITTEE_SIZE as usize * MAX_COMMITTEE_SIZE as usize;

/// The maximum number of connection attempts in an interval.
const MAX_CONNECTION_ATTEMPTS: usize = 10;
/// The maximum interval to restrict a peer.
const RESTRICTED_INTERVAL: i64 = (MAX_CONNECTION_ATTEMPTS as u64 * MAX_BATCH_DELAY / 1000) as i64; // seconds

/// Part of the Gateway API that deals with networking.
/// This is a separate trait to allow for easier testing/mocking.
pub trait Transport<N: Network>: Send + Sync {
    fn send(&self, peer_ip: SocketAddr, event: Event<N>);
    fn broadcast(&self, event: Event<N>);
}

#[derive(Clone)]
pub struct Gateway<N: Network> {
    /// The account of the node.
    account: Account<N>,
    /// The storage.
    storage: Storage<N>,
    /// The TCP stack.
    tcp: Tcp,
    /// The cache.
    cache: Arc<Cache<N>>,
    /// The resolver.
    resolver: Arc<Resolver<N>>,
    /// The map of connected peer IPs to their peer handlers.
    connected_peers: Arc<RwLock<IndexSet<SocketAddr>>>,
    /// The set of handshaking peers. While `Tcp` already recognizes the connecting IP addresses
    /// and prevents duplicate outbound connection attempts to the same IP address, it is unable to
    /// prevent simultaneous "two-way" connections between two peers (i.e. both nodes simultaneously
    /// attempt to connect to each other). This set is used to prevent this from happening.
    connecting_peers: Arc<Mutex<IndexSet<SocketAddr>>>,
    /// The primary sender.
    primary_sender: Arc<OnceCell<PrimarySender<N>>>,
    /// The worker senders.
    worker_senders: Arc<OnceCell<IndexMap<u8, WorkerSender<N>>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Gateway<N> {
    /// Initializes a new gateway.
    pub fn new(account: Account<N>, storage: Storage<N>, ip: Option<SocketAddr>, dev: Option<u16>) -> Result<Self> {
        // Initialize the gateway IP.
        let ip = match (ip, dev) {
            (_, Some(dev)) => SocketAddr::from_str(&format!("127.0.0.1:{}", MEMORY_POOL_PORT + dev))?,
            (None, None) => SocketAddr::from_str(&format!("0.0.0.0:{}", MEMORY_POOL_PORT))?,
            (Some(ip), None) => ip,
        };
        // Initialize the TCP stack.
        let tcp = Tcp::new(Config::new(ip, MAX_COMMITTEE_SIZE));
        // Return the gateway.
        Ok(Self {
            account,
            storage,
            tcp,
            cache: Default::default(),
            resolver: Default::default(),
            connected_peers: Default::default(),
            connecting_peers: Default::default(),
            primary_sender: Default::default(),
            worker_senders: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the gateway.
    pub async fn run(&self, worker_senders: IndexMap<u8, WorkerSender<N>>) {
        debug!("Starting the gateway for the memory pool...");

        // Set the worker senders.
        self.worker_senders.set(worker_senders).expect("The worker senders are already set");

        // Enable the TCP protocols.
        self.enable_handshake().await;
        self.enable_reading().await;
        self.enable_writing().await;
        self.enable_disconnect().await;
        self.enable_on_connect().await;
        // Enable the TCP listener. Note: This must be called after the above protocols.
        let _listening_addr = self.tcp.enable_listener().await.expect("Failed to enable the TCP listener");

        info!("Started the gateway for the memory pool at '{}'", self.local_ip());
    }

    /// Returns the account of the node.
    pub const fn account(&self) -> &Account<N> {
        &self.account
    }

    /// Returns the IP address of this node.
    pub fn local_ip(&self) -> SocketAddr {
        self.tcp.listening_addr().expect("The TCP listener is not enabled")
    }

    /// Returns `true` if the given IP is this node.
    pub fn is_local_ip(&self, ip: SocketAddr) -> bool {
        ip == self.local_ip()
            || (ip.ip().is_unspecified() || ip.ip().is_loopback()) && ip.port() == self.local_ip().port()
    }

    /// Returns the resolver.
    pub fn resolver(&self) -> &Resolver<N> {
        &self.resolver
    }

    /// Returns the primary sender.
    pub fn primary_sender(&self) -> &PrimarySender<N> {
        self.primary_sender.get().expect("Primary sender not set")
    }

    /// Sets the primary sender.
    pub fn set_primary_sender(&self, primary_sender: PrimarySender<N>) {
        self.primary_sender.set(primary_sender).expect("Primary sender already set");
    }

    /// Returns the number of workers.
    pub fn num_workers(&self) -> u8 {
        u8::try_from(self.worker_senders.get().expect("Missing worker senders in gateway").len())
            .expect("Too many workers")
    }

    /// Returns the worker sender for the given worker ID.
    pub fn get_worker_sender(&self, worker_id: u8) -> Option<&WorkerSender<N>> {
        self.worker_senders.get().and_then(|senders| senders.get(&worker_id))
    }

    /// Returns `true` if the node is connected to the given peer IP.
    pub fn is_connected(&self, ip: SocketAddr) -> bool {
        self.connected_peers.read().contains(&ip)
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
    pub fn connected_peers(&self) -> &RwLock<IndexSet<SocketAddr>> {
        &self.connected_peers
    }

    /// Attempts to connect to the given peer IP.
    pub fn connect(&self, peer_ip: SocketAddr) -> Option<JoinHandle<()>> {
        // Return early if the attempt is against the protocol rules.
        if let Err(forbidden_error) = self.check_connection_attempt(peer_ip) {
            warn!("{forbidden_error}");
            return None;
        }

        let self_ = self.clone();
        Some(tokio::spawn(async move {
            // Attempt to connect to the peer.
            if let Err(error) = self_.tcp.connect(peer_ip).await {
                self_.connecting_peers.lock().shift_remove(&peer_ip);
                warn!("Unable to connect to '{peer_ip}' - {error}");
            }
        }))
    }

    /// Ensure we are allowed to connect to the given peer.
    fn check_connection_attempt(&self, peer_ip: SocketAddr) -> Result<()> {
        // Ensure the peer IP is not this node.
        if self.is_local_ip(peer_ip) {
            bail!("{CONTEXT} Dropping connection attempt to '{peer_ip}' (attempted to self-connect)")
        }
        // Ensure the node does not surpass the maximum number of peer connections.
        if self.number_of_connected_peers() >= self.max_connected_peers() {
            bail!("{CONTEXT} Dropping connection attempt to '{peer_ip}' (maximum peers reached)")
        }
        // Ensure the node is not already connected to this peer.
        if self.is_connected(peer_ip) {
            bail!("{CONTEXT} Dropping connection attempt to '{peer_ip}' (already connected)")
        }
        // Ensure the node is not already connecting to this peer.
        if !self.connecting_peers.lock().insert(peer_ip) {
            bail!("{CONTEXT} Dropping connection attempt to '{peer_ip}' (already shaking hands as the initiator)")
        }
        Ok(())
    }

    /// Ensure the peer is allowed to connect.
    fn ensure_peer_is_allowed(&self, peer_ip: SocketAddr) -> Result<()> {
        // Ensure the peer IP is not this node.
        if self.is_local_ip(peer_ip) {
            bail!("{CONTEXT} Dropping connection request from '{peer_ip}' (attempted to self-connect)")
        }
        // Ensure the node is not already connecting to this peer.
        if !self.connecting_peers.lock().insert(peer_ip) {
            bail!("{CONTEXT} Dropping connection request from '{peer_ip}' (already shaking hands as the initiator)")
        }
        // Ensure the node is not already connected to this peer.
        if self.is_connected(peer_ip) {
            bail!("{CONTEXT} Dropping connection request from '{peer_ip}' (already connected)")
        }
        // Ensure the peer is not spamming connection attempts.
        if !peer_ip.ip().is_loopback() {
            // Add this connection attempt and retrieve the number of attempts.
            let num_attempts = self.cache.insert_inbound_connection(peer_ip.ip(), RESTRICTED_INTERVAL);
            // Ensure the connecting peer has not surpassed the connection attempt limit.
            if num_attempts > MAX_CONNECTION_ATTEMPTS {
                bail!("Dropping connection request from '{peer_ip}' (tried {num_attempts} times)")
            }
        }
        Ok(())
    }

    /// Inserts the given peer into the connected peers.
    fn insert_connected_peer(&self, peer_ip: SocketAddr, peer_addr: SocketAddr, address: Address<N>) {
        // Adds a bidirectional map between the listener address and (ambiguous) peer address.
        self.resolver.insert_peer(peer_ip, peer_addr, address);
        // Add an transmission for this peer in the connected peers.
        self.connected_peers.write().insert(peer_ip);
    }

    /// Removes the connected peer and adds them to the candidate peers.
    fn remove_connected_peer(&self, peer_ip: SocketAddr) {
        // Removes the bidirectional map between the listener address and (ambiguous) peer address.
        self.resolver.remove_peer(peer_ip);
        // Remove this peer from the connected peers, if it exists.
        self.connected_peers.write().shift_remove(&peer_ip);
    }

    /// Sends the given event to specified peer.
    ///
    /// This function returns as soon as the event is queued to be sent,
    /// without waiting for the actual delivery; instead, the caller is provided with a [`oneshot::Receiver`]
    /// which can be used to determine when and whether the event has been delivered.
    fn send_inner(&self, peer_ip: SocketAddr, event: Event<N>) -> Option<oneshot::Receiver<io::Result<()>>> {
        // Resolve the listener IP to the (ambiguous) peer address.
        let Some(peer_addr) = self.resolver.get_ambiguous(peer_ip) else {
            warn!("Unable to resolve the listener IP address '{peer_ip}'");
            return None;
        };
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
        let Some(peer_ip) = self.resolver.get_listener(peer_addr) else {
            bail!("{CONTEXT} Unable to resolve the (ambiguous) peer address '{peer_addr}'")
        };
        // Drop the peer, if they have exceeded the rate limit (i.e. they are requesting too much from us).
        let num_events = self.cache.insert_inbound_event(peer_ip, CACHE_EVENTS_INTERVAL);
        if num_events >= CACHE_EVENTS {
            bail!("Dropping '{peer_ip}' for spamming events (num_events = {num_events})")
        }
        // Rate limit for duplicate requests.
        if matches!(&event, &Event::CertificateRequest(_) | &Event::CertificateResponse(_)) {
            // Retrieve the certificate ID.
            let certificate_id = match &event {
                Event::CertificateRequest(CertificateRequest { certificate_id }) => *certificate_id,
                Event::CertificateResponse(CertificateResponse { certificate }) => certificate.certificate_id(),
                _ => unreachable!(),
            };
            // Skip processing this certificate if the rate limit was exceed (i.e. someone is spamming a specific certificate).
            let num_events = self.cache.insert_inbound_certificate(certificate_id, CACHE_REQUESTS_INTERVAL);
            if num_events >= CACHE_MAX_DUPLICATES {
                return Ok(());
            }
        } else if matches!(&event, &Event::TransmissionRequest(_) | Event::TransmissionResponse(_)) {
            // Retrieve the transmission ID.
            let transmission_id = match &event {
                Event::TransmissionRequest(TransmissionRequest { transmission_id }) => *transmission_id,
                Event::TransmissionResponse(TransmissionResponse { transmission_id, .. }) => *transmission_id,
                _ => unreachable!(),
            };
            // Skip processing this certificate if the rate limit was exceed (i.e. someone is spamming a specific certificate).
            let num_events = self.cache.insert_inbound_transmission(transmission_id, CACHE_REQUESTS_INTERVAL);
            if num_events >= CACHE_MAX_DUPLICATES {
                return Ok(());
            }
        }
        trace!("{CONTEXT} Received '{}' from '{peer_ip}'", event.name());

        // This match statement handles the inbound event by deserializing the event,
        // checking the event is valid, and then calling the appropriate (trait) handler.
        match event {
            Event::BatchPropose(batch_propose) => {
                // Send the batch propose to the primary.
                let _ = self.primary_sender().tx_batch_propose.send((peer_ip, batch_propose)).await;
                Ok(())
            }
            Event::BatchSignature(batch_signature) => {
                // Send the batch signature to the primary.
                let _ = self.primary_sender().tx_batch_signature.send((peer_ip, batch_signature)).await;
                Ok(())
            }
            Event::BatchCertified(batch_certified) => {
                // Send the batch certificate to the primary.
                let _ = self.primary_sender().tx_batch_certified.send((peer_ip, batch_certified.certificate)).await;
                Ok(())
            }
            Event::CertificateRequest(certificate_request) => {
                // Send the certificate request to the primary.
                let _ = self.primary_sender().tx_certificate_request.send((peer_ip, certificate_request)).await;
                Ok(())
            }
            Event::CertificateResponse(certificate_response) => {
                // Send the certificate response to the primary.
                let _ = self.primary_sender().tx_certificate_response.send((peer_ip, certificate_response)).await;
                Ok(())
            }
            Event::ChallengeRequest(..) | Event::ChallengeResponse(..) => {
                // Disconnect as the peer is not following the protocol.
                bail!("{CONTEXT} Peer '{peer_ip}' is not following the protocol")
            }
            Event::Disconnect(disconnect) => {
                bail!("{CONTEXT} Disconnecting peer '{peer_ip}' for the following reason: {:?}", disconnect.reason)
            }
            Event::TransmissionRequest(request) => {
                // TODO (howardwu): Add rate limiting checks on this event, on a per-peer basis.
                // Determine the worker ID.
                let Ok(worker_id) = assign_to_worker(request.transmission_id, self.num_workers()) else {
                    warn!("{CONTEXT} Unable to assign transmission ID '{}' to a worker", request.transmission_id);
                    return Ok(());
                };
                // Send the transmission request to the worker.
                if let Some(sender) = self.get_worker_sender(worker_id) {
                    // Send the transmission request to the worker.
                    let _ = sender.tx_transmission_request.send((peer_ip, request)).await;
                }
                Ok(())
            }
            Event::TransmissionResponse(response) => {
                // Determine the worker ID.
                let Ok(worker_id) = assign_to_worker(response.transmission_id, self.num_workers()) else {
                    warn!("{CONTEXT} Unable to assign transmission ID '{}' to a worker", response.transmission_id);
                    return Ok(());
                };
                // Send the transmission response to the worker.
                if let Some(sender) = self.get_worker_sender(worker_id) {
                    // Send the transmission response to the worker.
                    let _ = sender.tx_transmission_response.send((peer_ip, response)).await;
                }
                Ok(())
            }
            Event::WorkerPing(ping) => {
                let num_workers = self.num_workers();
                for transmission_id in ping.transmission_ids {
                    // Determine the worker ID.
                    let Ok(worker_id) = assign_to_worker(transmission_id, num_workers) else {
                        warn!("{CONTEXT} Unable to assign transmission ID '{transmission_id}' to a worker");
                        continue;
                    };
                    // Send the transmission ID to the worker.
                    if let Some(sender) = self.get_worker_sender(worker_id) {
                        // Send the transmission ID to the worker.
                        let _ = sender.tx_worker_ping.send((peer_ip, transmission_id)).await;
                    }
                }
                Ok(())
            }
        }
    }

    /// Disconnects from the given peer IP, if the peer is connected.
    pub fn disconnect(&self, peer_ip: SocketAddr) -> JoinHandle<()> {
        let gateway = self.clone();
        tokio::spawn(async move {
            if let Some(peer_addr) = gateway.resolver.get_ambiguous(peer_ip) {
                // Disconnect from this peer.
                let _disconnected = gateway.tcp.disconnect(peer_addr).await;
                debug_assert!(_disconnected);
            }
        })
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    #[allow(dead_code)]
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the gateway.
    pub async fn shut_down(&self) {
        trace!("Shutting down the gateway...");
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
        // Close the listener.
        self.tcp.shut_down().await;
    }
}

impl<N: Network> Transport<N> for Gateway<N> {
    /// Sends the given event to specified peer.
    ///
    /// This method is rate limited to prevent spamming the peer.
    fn send(&self, peer_ip: SocketAddr, event: Event<N>) {
        macro_rules! send {
            ($self:ident, $cache_map:ident, $interval:expr, $freq:expr) => {
                let self_ = $self.clone();
                tokio::spawn(async move {
                    // Rate limit the number of certificate requests sent to the peer.
                    while self_.cache.$cache_map(peer_ip, $interval) > $freq {
                        // Sleep for a short period of time to allow the cache to clear.
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    // Send the event to the peer.
                    self_.send_inner(peer_ip, event);
                });
            };
        }

        // If the event type is a certificate request, increment the cache.
        if matches!(event, Event::CertificateRequest(_)) | matches!(event, Event::CertificateResponse(_)) {
            // Update the outbound event cache. This is necessary to ensure we don't under count the outbound events.
            self.cache.insert_outbound_event(peer_ip, CACHE_EVENTS_INTERVAL);
            // Send the event to the peer.
            send!(self, insert_outbound_certificate, CACHE_REQUESTS_INTERVAL, CACHE_CERTIFICATES);
        }
        // If the event type is a transmission request, increment the cache.
        else if matches!(event, Event::TransmissionRequest(_)) | matches!(event, Event::TransmissionResponse(_)) {
            // Update the outbound event cache. This is necessary to ensure we don't under count the outbound events.
            self.cache.insert_outbound_event(peer_ip, CACHE_EVENTS_INTERVAL);
            // Send the event to the peer.
            send!(self, insert_outbound_transmission, CACHE_REQUESTS_INTERVAL, CACHE_TRANSMISSIONS);
        }
        // Otherwise, employ a general rate limit.
        else {
            // Send the event to the peer.
            send!(self, insert_outbound_event, CACHE_EVENTS_INTERVAL, CACHE_EVENTS);
        }
    }

    /// Broadcasts the given event to all connected peers.
    // TODO(ljedrz): the event should be checked for the presence of Data::Object, and
    // serialized in advance if it's there.
    fn broadcast(&self, event: Event<N>) {
        // Ensure there are connected peers.
        if self.number_of_connected_peers() > 0 {
            // Iterate through all connected peers.
            for peer_ip in self.connected_peers.read().iter() {
                // Send the event to the peer.
                self.send(*peer_ip, event.clone());
            }
        }
    }
}

impl<N: Network> P2P for Gateway<N> {
    /// Returns a reference to the TCP instance.
    fn tcp(&self) -> &Tcp {
        &self.tcp
    }
}

#[async_trait]
impl<N: Network> Reading for Gateway<N> {
    type Codec = EventCodec<N>;
    type Message = Event<N>;

    /// The maximum queue depth of incoming messages for a single peer.
    const MESSAGE_QUEUE_DEPTH: usize = CACHE_TRANSMISSIONS;

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, _peer_addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }

    /// Processes a message received from the network.
    async fn process_message(&self, peer_addr: SocketAddr, message: Self::Message) -> io::Result<()> {
        // Process the message. Disconnect if the peer violated the protocol.
        if let Err(error) = self.inbound(peer_addr, message).await {
            if let Some(peer_ip) = self.resolver.get_listener(peer_addr) {
                warn!("{CONTEXT} Disconnecting from '{peer_ip}' - {error}");
                self.send(peer_ip, Event::Disconnect(DisconnectReason::ProtocolViolation.into()));
                // Disconnect from this peer.
                self.disconnect(peer_ip);
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<N: Network> Writing for Gateway<N> {
    type Codec = EventCodec<N>;
    type Message = Event<N>;

    /// The maximum queue depth of outgoing messages for a single peer.
    const MESSAGE_QUEUE_DEPTH: usize = CACHE_TRANSMISSIONS;

    /// Creates an [`Encoder`] used to write the outbound messages to the target stream.
    /// The `side` parameter indicates the connection side **from the node's perspective**.
    fn codec(&self, _addr: SocketAddr, _side: ConnectionSide) -> Self::Codec {
        Default::default()
    }
}

#[async_trait]
impl<N: Network> Disconnect for Gateway<N> {
    /// Any extra operations to be performed during a disconnect.
    async fn handle_disconnect(&self, peer_addr: SocketAddr) {
        if let Some(peer_ip) = self.resolver.get_listener(peer_addr) {
            self.remove_connected_peer(peer_ip);
        }
    }
}

#[async_trait]
impl<N: Network> OnConnect for Gateway<N> {
    async fn on_connect(&self, _peer_addr: SocketAddr) {
        return;
    }
}

#[async_trait]
impl<N: Network> Handshake for Gateway<N> {
    /// Performs the handshake protocol.
    async fn perform_handshake(&self, mut connection: Connection) -> io::Result<Connection> {
        // Perform the handshake.
        let peer_addr = connection.addr();
        let peer_side = connection.side();
        let stream = self.borrow_stream(&mut connection);

        // If this is an inbound connection, we log it, but don't know the listening address yet.
        // Otherwise, we can immediately register the listening address.
        let mut peer_ip = if peer_side == ConnectionSide::Initiator {
            debug!("{CONTEXT} Gateway received a connection request from '{peer_addr}'");
            None
        } else {
            debug!("{CONTEXT} Gateway is connecting to {peer_addr}...");
            Some(peer_addr)
        };

        // Perform the handshake; we pass on a mutable reference to peer_ip in case the process is broken at any point in time.
        let handshake_result = if peer_side == ConnectionSide::Responder {
            self.handshake_inner_initiator(peer_addr, &mut peer_ip, stream).await
        } else {
            self.handshake_inner_responder(peer_addr, &mut peer_ip, stream).await
        };

        // Remove the address from the collection of connecting peers (if the handshake got to the point where it's known).
        if let Some(ip) = peer_ip {
            self.connecting_peers.lock().shift_remove(&ip);
        }

        // If the handshake succeeded, announce it.
        if let Ok((ref peer_ip, _)) = handshake_result {
            info!("{CONTEXT} Gateway is connected to '{peer_ip}'");
        }

        Ok(connection)
    }
}

/// A macro unwrapping the expected handshake event or returning an error for unexpected events.
macro_rules! expect_event {
    ($event_ty:path, $framed:expr, $peer_addr:expr) => {
        match $framed.try_next().await? {
            // Received the expected event, proceed.
            Some($event_ty(data)) => {
                trace!("{CONTEXT} Gateway received '{}' from '{}'", data.name(), $peer_addr);
                data
            }
            // Received a disconnect event, abort.
            Some(Event::Disconnect(reason)) => {
                return Err(error(format!("{CONTEXT} '{}' disconnected: {reason:?}", $peer_addr)));
            }
            // Received an unexpected event, abort.
            Some(ty) => {
                return Err(error(format!(
                    "{CONTEXT} '{}' did not follow the handshake protocol: received {:?} instead of {}",
                    $peer_addr,
                    ty.name(),
                    stringify!($event_ty),
                )))
            }
            // Received nothing.
            None => {
                return Err(error(format!(
                    "{CONTEXT} '{}' disconnected before sending {:?}",
                    $peer_addr,
                    stringify!($event_ty)
                )))
            }
        }
    };
}

/// Send the given message to the peer.
async fn send<N: Network>(
    framed: &mut Framed<&mut TcpStream, EventCodec<N>>,
    peer_addr: SocketAddr,
    event: Event<N>,
) -> io::Result<()> {
    trace!("{CONTEXT} Gateway is sending '{}' to '{peer_addr}'", event.name());
    framed.send(event).await
}

impl<N: Network> Gateway<N> {
    /// The connection initiator side of the handshake.
    async fn handshake_inner_initiator<'a>(
        &'a self,
        peer_addr: SocketAddr,
        peer_ip: &mut Option<SocketAddr>,
        stream: &'a mut TcpStream,
    ) -> io::Result<(SocketAddr, Framed<&mut TcpStream, EventCodec<N>>)> {
        // This value is immediately guaranteed to be present, so it can be unwrapped.
        let peer_ip = peer_ip.unwrap();
        // Construct the stream.
        let mut framed = Framed::new(stream, EventCodec::<N>::handshake());

        // Initialize an RNG.
        let rng = &mut rand::rngs::OsRng;

        /* Step 1: Send the challenge request. */

        // Sample a random nonce.
        let our_nonce = rng.gen();
        // Send a challenge request to the peer.
        let our_request = ChallengeRequest::new(self.local_ip().port(), self.account.address(), our_nonce);
        send(&mut framed, peer_addr, Event::ChallengeRequest(our_request)).await?;

        /* Step 2: Receive the peer's challenge response followed by the challenge request. */

        // Listen for the challenge response message.
        let peer_response = expect_event!(Event::ChallengeResponse, framed, peer_addr);
        // Listen for the challenge request message.
        let peer_request = expect_event!(Event::ChallengeRequest, framed, peer_addr);

        // Verify the challenge response. If a disconnect reason was returned, send the disconnect message and abort.
        if let Some(reason) =
            self.verify_challenge_response(peer_addr, peer_request.address, peer_response, our_nonce).await
        {
            send(&mut framed, peer_addr, reason.into()).await?;
            return Err(error(format!("Dropped '{peer_addr}' for reason: {reason:?}")));
        }
        // Verify the challenge request. If a disconnect reason was returned, send the disconnect message and abort.
        if let Some(reason) = self.verify_challenge_request(peer_addr, &peer_request) {
            send(&mut framed, peer_addr, reason.into()).await?;
            return Err(error(format!("Dropped '{peer_addr}' for reason: {reason:?}")));
        }

        /* Step 3: Send the challenge response. */

        // Sign the counterparty nonce.
        let Ok(our_signature) = self.account.sign_bytes(&peer_request.nonce.to_le_bytes(), rng) else {
            return Err(error(format!("Failed to sign the challenge request nonce from '{peer_addr}'")));
        };
        // Send the challenge response.
        let our_response = ChallengeResponse { signature: Data::Object(our_signature) };
        send(&mut framed, peer_addr, Event::ChallengeResponse(our_response)).await?;

        // Add the peer to the gateway.
        self.insert_connected_peer(peer_ip, peer_addr, peer_request.address);

        Ok((peer_ip, framed))
    }

    /// The connection responder side of the handshake.
    async fn handshake_inner_responder<'a>(
        &'a self,
        peer_addr: SocketAddr,
        peer_ip: &mut Option<SocketAddr>,
        stream: &'a mut TcpStream,
    ) -> io::Result<(SocketAddr, Framed<&mut TcpStream, EventCodec<N>>)> {
        // Construct the stream.
        let mut framed = Framed::new(stream, EventCodec::<N>::handshake());

        /* Step 1: Receive the challenge request. */

        // Listen for the challenge request message.
        let peer_request = expect_event!(Event::ChallengeRequest, framed, peer_addr);

        // Obtain the peer's listening address.
        *peer_ip = Some(SocketAddr::new(peer_addr.ip(), peer_request.listener_port));
        let peer_ip = peer_ip.unwrap();

        // Knowing the peer's listening address, ensure it is allowed to connect.
        if let Err(forbidden_message) = self.ensure_peer_is_allowed(peer_ip) {
            return Err(error(format!("{forbidden_message}")));
        }
        // Verify the challenge request. If a disconnect reason was returned, send the disconnect message and abort.
        if let Some(reason) = self.verify_challenge_request(peer_addr, &peer_request) {
            send(&mut framed, peer_addr, reason.into()).await?;
            return Err(error(format!("Dropped '{peer_addr}' for reason: {reason:?}")));
        }

        /* Step 2: Send the challenge response followed by own challenge request. */

        // Initialize an RNG.
        let rng = &mut rand::rngs::OsRng;

        // Sign the counterparty nonce.
        let Ok(our_signature) = self.account.sign_bytes(&peer_request.nonce.to_le_bytes(), rng) else {
            return Err(error(format!("Failed to sign the challenge request nonce from '{peer_addr}'")));
        };
        // Send the challenge response.
        let our_response = ChallengeResponse { signature: Data::Object(our_signature) };
        send(&mut framed, peer_addr, Event::ChallengeResponse(our_response)).await?;

        // Sample a random nonce.
        let our_nonce = rng.gen();
        // Send the challenge request.
        let our_request = ChallengeRequest::new(self.local_ip().port(), self.account.address(), our_nonce);
        send(&mut framed, peer_addr, Event::ChallengeRequest(our_request)).await?;

        /* Step 3: Receive the challenge response. */

        // Listen for the challenge response message.
        let peer_response = expect_event!(Event::ChallengeResponse, framed, peer_addr);
        // Verify the challenge response. If a disconnect reason was returned, send the disconnect message and abort.
        if let Some(reason) =
            self.verify_challenge_response(peer_addr, peer_request.address, peer_response, our_nonce).await
        {
            send(&mut framed, peer_addr, reason.into()).await?;
            return Err(error(format!("Dropped '{peer_addr}' for reason: {reason:?}")));
        }
        // Add the peer to the gateway.
        self.insert_connected_peer(peer_ip, peer_addr, peer_request.address);

        Ok((peer_ip, framed))
    }

    /// Verifies the given challenge request. Returns a disconnect reason if the request is invalid.
    fn verify_challenge_request(&self, peer_addr: SocketAddr, event: &ChallengeRequest<N>) -> Option<DisconnectReason> {
        // Retrieve the components of the challenge request.
        let &ChallengeRequest { version, listener_port: _, address, nonce: _ } = event;
        // Ensure the event protocol version is not outdated.
        if version < Event::<N>::VERSION {
            warn!("{CONTEXT} Gateway is dropping '{peer_addr}' on version {version} (outdated)");
            return Some(DisconnectReason::OutdatedClientVersion);
        }
        // TODO (howardwu): Remove this check, instead checking the address is unique.
        //  Then, later on, use the committee object to perform filtering of all active connections.
        // Ensure the address is in the committee.
        if !self.storage.current_committee().is_committee_member(address) {
            warn!("{CONTEXT} Gateway is dropping '{peer_addr}' for an invalid address ({address})");
            return Some(DisconnectReason::ProtocolViolation);
        }
        None
    }

    /// Verifies the given challenge response. Returns a disconnect reason if the response is invalid.
    async fn verify_challenge_response(
        &self,
        peer_addr: SocketAddr,
        peer_address: Address<N>,
        response: ChallengeResponse<N>,
        expected_nonce: u64,
    ) -> Option<DisconnectReason> {
        // Retrieve the components of the challenge response.
        let ChallengeResponse { signature } = response;
        // Perform the deferred non-blocking deserialization of the signature.
        let Ok(Ok(signature)) = task::spawn_blocking(move || signature.deserialize_blocking()).await else {
            warn!("{CONTEXT} Gateway handshake with '{peer_addr}' failed (cannot deserialize the signature)");
            return Some(DisconnectReason::InvalidChallengeResponse);
        };
        // Verify the signature.
        if !signature.verify_bytes(&peer_address, &expected_nonce.to_le_bytes()) {
            warn!("{CONTEXT} Gateway handshake with '{peer_addr}' failed (invalid signature)");
            return Some(DisconnectReason::InvalidChallengeResponse);
        }
        None
    }
}

#[cfg(test)]
pub mod prop_tests {
    use crate::{helpers::init_worker_channels, Gateway, Worker, MAX_WORKERS, MEMORY_POOL_PORT};
    use indexmap::IndexMap;
    use proptest::{
        prelude::{any, any_with, Arbitrary, BoxedStrategy, Just, Strategy},
        sample::Selector,
    };
    use snarkos_node_tcp::P2P;
    use snarkvm::prelude::Testnet3;
    use std::{
        fmt::{Debug, Formatter},
        net::{IpAddr, Ipv4Addr, SocketAddr},
        sync::Arc,
    };
    use test_strategy::proptest;

    type CurrentNetwork = Testnet3;

    use crate::{
        helpers::Storage,
        prop_tests::GatewayAddress::{Dev, Prod},
    };
    use snarkos_account::Account;
    use snarkos_node_narwhal_committee::{
        prop_tests::{CommitteeContext, ValidatorSet},
        MAX_COMMITTEE_SIZE,
    };
    use snarkos_node_narwhal_ledger_service::MockLedgerService;

    impl Debug for Gateway<CurrentNetwork> {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            // TODO implement Debug properly and move it over to production code
            f.debug_tuple("Gateway").field(&self.account.address()).field(&self.tcp.config()).finish()
        }
    }

    #[derive(Debug, test_strategy::Arbitrary)]
    enum GatewayAddress {
        Dev(u8),
        Prod(Option<SocketAddr>),
    }

    impl GatewayAddress {
        fn ip(&self) -> Option<SocketAddr> {
            if let GatewayAddress::Prod(ip) = self {
                return *ip;
            }
            None
        }

        fn port(&self) -> Option<u16> {
            if let GatewayAddress::Dev(port) = self {
                return Some(*port as u16);
            }
            None
        }
    }

    impl Arbitrary for Gateway<CurrentNetwork> {
        type Parameters = ();
        type Strategy = BoxedStrategy<Gateway<CurrentNetwork>>;

        fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
            any_valid_dev_gateway()
                .prop_map(|(storage, _, account, address)| {
                    Gateway::new(account, storage, address.ip(), address.port()).unwrap()
                })
                .boxed()
        }
    }

    type GatewayInput = (Storage<CurrentNetwork>, CommitteeContext, Account<CurrentNetwork>, GatewayAddress);

    fn any_valid_dev_gateway() -> BoxedStrategy<GatewayInput> {
        (any::<CommitteeContext>(), any::<Selector>())
            .prop_flat_map(|(context, account_selector)| {
                let CommitteeContext(_, ValidatorSet(validators)) = context.clone();
                (
                    any_with::<Storage<CurrentNetwork>>(context.clone()),
                    Just(context),
                    Just(account_selector.select(validators)),
                    0u8..,
                )
                    .prop_map(|(a, b, c, d)| (a, b, c.account, Dev(d)))
            })
            .boxed()
    }

    fn any_valid_prod_gateway() -> BoxedStrategy<GatewayInput> {
        (any::<CommitteeContext>(), any::<Selector>())
            .prop_flat_map(|(context, account_selector)| {
                let CommitteeContext(_, ValidatorSet(validators)) = context.clone();
                (
                    any_with::<Storage<CurrentNetwork>>(context.clone()),
                    Just(context),
                    Just(account_selector.select(validators)),
                    any::<Option<SocketAddr>>(),
                )
                    .prop_map(|(a, b, c, d)| (a, b, c.account, Prod(d)))
            })
            .boxed()
    }

    #[proptest]
    fn gateway_dev_initialization(#[strategy(any_valid_dev_gateway())] input: GatewayInput) {
        let (storage, _, account, dev) = input;
        let address = account.address();
        let gateway = Gateway::new(account, storage, dev.ip(), dev.port()).unwrap();
        let tcp_config = gateway.tcp().config();
        assert_eq!(tcp_config.listener_ip, Some(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert_eq!(tcp_config.desired_listening_port, Some(MEMORY_POOL_PORT + dev.port().unwrap()));

        let tcp_config = gateway.tcp().config();
        assert_eq!(tcp_config.max_connections, MAX_COMMITTEE_SIZE);
        assert_eq!(gateway.account().address(), address);
    }

    #[proptest]
    fn gateway_prod_initialization(#[strategy(any_valid_prod_gateway())] input: GatewayInput) {
        let (storage, _, account, dev) = input;
        let address = account.address();
        let gateway = Gateway::new(account, storage, dev.ip(), dev.port()).unwrap();
        let tcp_config = gateway.tcp().config();
        if let Some(socket_addr) = dev.ip() {
            assert_eq!(tcp_config.listener_ip, Some(socket_addr.ip()));
            assert_eq!(tcp_config.desired_listening_port, Some(socket_addr.port()));
        } else {
            assert_eq!(tcp_config.listener_ip, Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
            assert_eq!(tcp_config.desired_listening_port, Some(MEMORY_POOL_PORT));
        }

        let tcp_config = gateway.tcp().config();
        assert_eq!(tcp_config.max_connections, MAX_COMMITTEE_SIZE);
        assert_eq!(gateway.account().address(), address);
    }

    #[proptest(async = "tokio")]
    async fn gateway_start(
        #[strategy(any_valid_dev_gateway())] input: GatewayInput,
        #[strategy(0..MAX_WORKERS)] workers_count: u8,
    ) {
        let (storage, _, account, dev) = input;
        let worker_storage = storage.clone();
        let gateway = Gateway::new(account, storage, dev.ip(), dev.port()).unwrap();

        let (workers, worker_senders) = {
            // Construct a map of the worker senders.
            let mut tx_workers = IndexMap::new();
            let mut workers = IndexMap::new();

            // Initialize the workers.
            for id in 0..workers_count {
                // Construct the worker channels.
                let (tx_worker, rx_worker) = init_worker_channels();
                // Construct the worker instance.
                let ledger = Arc::new(MockLedgerService::new());
                let worker =
                    Worker::new(id, Arc::new(gateway.clone()), worker_storage.clone(), ledger, Default::default())
                        .unwrap();
                // Run the worker instance.
                worker.run(rx_worker);

                // Add the worker and the worker sender to maps
                workers.insert(id, worker);
                tx_workers.insert(id, tx_worker);
            }
            (workers, tx_workers)
        };
        gateway.run(worker_senders).await;
        assert_eq!(
            gateway.local_ip(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), MEMORY_POOL_PORT + dev.port().unwrap())
        );
        assert_eq!(gateway.num_workers(), workers.len() as u8);
    }
}
