// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use std::{
    collections::HashSet,
    io,
    net::SocketAddr,
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering::*},
        Arc,
    },
};

use parking_lot::Mutex;
use tokio::{
    io::split,
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};
use tracing::*;

use crate::{
    connections::{Connection, ConnectionSide, Connections},
    protocols::{Protocol, Protocols},
    Config,
    KnownPeers,
    Stats,
};

macro_rules! enable_protocol {
    ($handler_type: ident, $node:expr, $conn: expr) => {
        if let Some(handler) = $node.protocols.$handler_type.get() {
            let (conn_returner, conn_retriever) = oneshot::channel();

            handler.trigger(($conn, conn_returner));

            match conn_retriever.await {
                Ok(Ok(conn)) => conn,
                Err(_) => return Err(io::ErrorKind::BrokenPipe.into()),
                Ok(e) => return e,
            }
        } else {
            $conn
        }
    };
}

// A seuential numeric identifier assigned to `Tcp`s that were not provided with a name.
static SEQUENTIAL_NODE_ID: AtomicUsize = AtomicUsize::new(0);

/// The central object responsible for handling connections.
#[derive(Clone)]
pub struct Tcp(Arc<InnerTcp>);

impl Deref for Tcp {
    type Target = Arc<InnerTcp>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[doc(hidden)]
pub struct InnerTcp {
    /// The tracing span.
    span: Span,
    /// The node's configuration.
    config: Config,
    /// The node's listening address.
    listening_addr: Option<SocketAddr>,
    /// Contains objects used by the protocols implemented by the node.
    pub(crate) protocols: Protocols,
    /// A list of connections that have not been finalized yet.
    connecting: Mutex<HashSet<SocketAddr>>,
    /// Contains objects related to the node's active connections.
    connections: Connections,
    /// Collects statistics related to the node's peers.
    known_peers: KnownPeers,
    /// Collects statistics related to the node itself.
    stats: Stats,
    /// The node's tasks.
    pub(crate) tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl Tcp {
    /// Creates a new [`Tcp`] using the given [`Config`].
    pub async fn new(mut config: Config) -> io::Result<Self> {
        // if there is no pre-configured name, assign a sequential numeric identifier
        if config.name.is_none() {
            config.name = Some(SEQUENTIAL_NODE_ID.fetch_add(1, SeqCst).to_string());
        }

        // create a tracing span containing the node's name
        let span = create_span(config.name.as_deref().unwrap());

        // procure a listening address
        let listener = if let Some(listener_ip) = config.listener_ip {
            let listener = if let Some(port) = config.desired_listening_port {
                let desired_listening_addr = SocketAddr::new(listener_ip, port);
                match TcpListener::bind(desired_listening_addr).await {
                    Ok(listener) => listener,
                    Err(e) => {
                        if config.allow_random_port {
                            warn!(parent: &span, "trying any port, the desired one is unavailable: {}", e);
                            let random_available_addr = SocketAddr::new(listener_ip, 0);
                            TcpListener::bind(random_available_addr).await?
                        } else {
                            error!(parent: &span, "the desired port is unavailable: {}", e);
                            return Err(e);
                        }
                    }
                }
            } else if config.allow_random_port {
                let random_available_addr = SocketAddr::new(listener_ip, 0);
                TcpListener::bind(random_available_addr).await?
            } else {
                panic!("you must either provide a desired port or allow a random one");
            };

            Some(listener)
        } else {
            None
        };

        let listening_addr = if let Some(ref listener) = listener {
            let ip = config.listener_ip.unwrap(); // safe; listener.is_some() => config.listener_ip.is_some()
            let port = listener.local_addr()?.port(); // discover the port if it was unspecified
            Some((ip, port).into())
        } else {
            None
        };

        let node = Tcp(Arc::new(InnerTcp {
            span,
            config,
            listening_addr,
            protocols: Default::default(),
            connecting: Default::default(),
            connections: Default::default(),
            known_peers: Default::default(),
            stats: Default::default(),
            tasks: Default::default(),
        }));

        if let Some(listener) = listener {
            // use a channel to know when the listening task is ready
            let (tx, rx) = oneshot::channel();

            let node_clone = node.clone();
            let listening_task = tokio::spawn(async move {
                trace!(parent: node_clone.span(), "spawned the listening task");
                tx.send(()).unwrap(); // safe; the channel was just opened

                loop {
                    match listener.accept().await {
                        Ok((stream, addr)) => {
                            debug!(parent: node_clone.span(), "tentatively accepted a connection from {}", addr);

                            if !node_clone.can_add_connection() {
                                debug!(parent: node_clone.span(), "rejecting the connection from {}", addr);
                                continue;
                            }

                            node_clone.connecting.lock().insert(addr);

                            let node_clone2 = node_clone.clone();
                            tokio::spawn(async move {
                                if let Err(e) = node_clone2.adapt_stream(stream, addr, ConnectionSide::Responder).await
                                {
                                    node_clone2.connecting.lock().remove(&addr);
                                    node_clone2.known_peers().register_failure(addr);
                                    error!(parent: node_clone2.span(), "couldn't accept a connection: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!(parent: node_clone.span(), "couldn't accept a connection: {}", e);
                        }
                    }
                }
            });
            node.tasks.lock().push(listening_task);
            let _ = rx.await;
            debug!(parent: node.span(), "listening on {}", node.listening_addr.unwrap());
        }

        debug!(parent: node.span(), "the node is ready");

        Ok(node)
    }

    /// Returns the name assigned to Tcp.
    #[inline]
    pub fn name(&self) -> &str {
        // safe; can be set as None in Config, but receives a default value on Tcp creation
        self.config.name.as_deref().unwrap()
    }

    /// Returns a reference to the Tcp's config.
    #[inline]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a reference to the Tcp's stats.
    #[inline]
    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    /// Returns the tracing [`Span`] associated with Tcp.
    #[inline]
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// Returns the Tcp's listening address; returns an error if Tcp was configured
    /// to not listen for inbound connections.
    pub fn listening_addr(&self) -> io::Result<SocketAddr> {
        self.listening_addr.ok_or_else(|| io::ErrorKind::AddrNotAvailable.into())
    }

    async fn enable_protocols(&self, conn: Connection) -> io::Result<Connection> {
        let mut conn = enable_protocol!(handshake, self, conn);

        // split the stream after the handshake (if not done before)
        if let Some(stream) = conn.stream.take() {
            let (reader, writer) = split(stream);
            conn.reader = Some(Box::new(reader));
            conn.writer = Some(Box::new(writer));
        }

        let conn = enable_protocol!(reading, self, conn);
        let conn = enable_protocol!(writing, self, conn);

        Ok(conn)
    }

    /// Prepares the freshly acquired connection to handle the protocols the Tcp implements.
    async fn adapt_stream(&self, stream: TcpStream, peer_addr: SocketAddr, own_side: ConnectionSide) -> io::Result<()> {
        self.known_peers.add(peer_addr);

        // register the port seen by the peer
        if own_side == ConnectionSide::Initiator {
            if let Ok(addr) = stream.local_addr() {
                debug!(
                    parent: self.span(), "establishing connection with {}; the peer is connected on port {}",
                    peer_addr, addr.port()
                );
            } else {
                warn!(parent: self.span(), "couldn't determine the peer's port");
            }
        }

        let connection = Connection::new(peer_addr, stream, !own_side);

        // enact the enabled protocols
        let mut connection = self.enable_protocols(connection).await?;

        // if Reading is enabled, we'll notify the related task when the connection is fully ready
        let conn_ready_tx = connection.readiness_notifier.take();

        self.connections.add(connection);
        self.connecting.lock().remove(&peer_addr);

        // send the aforementioned notification so that reading from the socket can commence
        if let Some(tx) = conn_ready_tx {
            let _ = tx.send(());
        }

        Ok(())
    }

    /// Connects to the provided `SocketAddr`.
    pub async fn connect(&self, addr: SocketAddr) -> io::Result<()> {
        if let Ok(listening_addr) = self.listening_addr() {
            if addr == listening_addr || addr.ip().is_loopback() && addr.port() == listening_addr.port() {
                error!(parent: self.span(), "can't connect to Tcp's own listening address ({})", addr);
                return Err(io::ErrorKind::AddrInUse.into());
            }
        }

        if !self.can_add_connection() {
            error!(parent: self.span(), "too many connections; refusing to connect to {}", addr);
            return Err(io::ErrorKind::PermissionDenied.into());
        }

        if self.connections.is_connected(addr) {
            warn!(parent: self.span(), "already connected to {}", addr);
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        if !self.connecting.lock().insert(addr) {
            warn!(parent: self.span(), "already connecting to {}", addr);
            return Err(io::ErrorKind::AlreadyExists.into());
        }

        let stream = TcpStream::connect(addr).await.map_err(|e| {
            self.connecting.lock().remove(&addr);
            e
        })?;

        let ret = self.adapt_stream(stream, addr, ConnectionSide::Initiator).await;

        if let Err(ref e) = ret {
            self.connecting.lock().remove(&addr);
            self.known_peers().register_failure(addr);
            error!(parent: self.span(), "couldn't initiate a connection with {}: {}", addr, e);
        }

        ret
    }

    /// Disconnects from the provided `SocketAddr`.
    pub async fn disconnect(&self, addr: SocketAddr) -> bool {
        if let Some(handler) = self.protocols.disconnect.get() {
            if self.is_connected(addr) {
                let (sender, receiver) = oneshot::channel();

                handler.trigger((addr, sender));
                let _ = receiver.await; // can't really fail
            }
        }

        let conn = self.connections.remove(addr);

        if let Some(ref conn) = conn {
            debug!(parent: self.span(), "disconnecting from {}", conn.addr());

            // shut the associated tasks down
            for task in conn.tasks.iter().rev() {
                task.abort();
            }

            // if the (owning) Tcp was not the initiator of the connection, it doesn't know the listening address
            // of the associated peer, so the related stats are unreliable; the next connection initiated by the
            // peer could be bound to an entirely different port number
            if conn.side() == ConnectionSide::Initiator {
                self.known_peers().remove(conn.addr());
            }

            debug!(parent: self.span(), "disconnected from {}", addr);
        } else {
            warn!(parent: self.span(), "wasn't connected to {}", addr);
        }

        conn.is_some()
    }

    /// Returns a list containing addresses of active connections.
    pub fn connected_addrs(&self) -> Vec<SocketAddr> {
        self.connections.addrs()
    }

    /// Returns a reference to the collection of statistics of Tcp's known peers.
    #[inline]
    pub fn known_peers(&self) -> &KnownPeers {
        &self.known_peers
    }

    /// Checks whether the provided address is connected.
    pub fn is_connected(&self, addr: SocketAddr) -> bool {
        self.connections.is_connected(addr)
    }

    /// Checks if Tcp is currently setting up a connection with the provided address.
    pub fn is_connecting(&self, addr: SocketAddr) -> bool {
        self.connecting.lock().contains(&addr)
    }

    /// Returns the number of active connections.
    pub fn num_connected(&self) -> usize {
        self.connections.num_connected()
    }

    /// Returns the number of connections that are currently being set up.
    pub fn num_connecting(&self) -> usize {
        self.connecting.lock().len()
    }

    /// Checks whether the `Tcp` can handle an additional connection.
    fn can_add_connection(&self) -> bool {
        let num_connected = self.num_connected();
        let limit = self.config.max_connections as usize;
        if num_connected >= limit || num_connected + self.num_connecting() >= limit {
            warn!(parent: self.span(), "maximum number of connections ({}) reached", limit);
            false
        } else {
            true
        }
    }

    /// Gracefully shuts Tcp down.
    pub async fn shut_down(&self) {
        debug!(parent: self.span(), "shutting down");

        let mut tasks = std::mem::take(&mut *self.tasks.lock()).into_iter();
        if let Some(listening_task) = tasks.next() {
            listening_task.abort(); // abort the listening task first
        }

        for addr in self.connected_addrs() {
            self.disconnect(addr).await;
        }

        for handle in tasks {
            handle.abort();
        }
    }
}

// FIXME: this can probably be done more elegantly
/// Creates the Tcp's tracing span based on its name.
fn create_span(tcp_name: &str) -> Span {
    let mut span = trace_span!("tcp", name = tcp_name);
    if !span.is_disabled() {
        return span;
    } else {
        span = debug_span!("tcp", name = tcp_name);
    }
    if !span.is_disabled() {
        return span;
    } else {
        span = info_span!("tcp", name = tcp_name);
    }
    if !span.is_disabled() {
        return span;
    } else {
        span = warn_span!("tcp", name = tcp_name);
    }
    if !span.is_disabled() { span } else { error_span!("tcp", name = tcp_name) }
}
