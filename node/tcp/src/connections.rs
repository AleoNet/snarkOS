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

//! Objects associated with connection handling.

use std::{collections::HashMap, net::SocketAddr, ops::Not};

use parking_lot::RwLock;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::oneshot,
    task::JoinHandle,
};

#[cfg(doc)]
use crate::protocols::{Handshake, Reading, Writing};

#[derive(Default)]
pub(crate) struct Connections(RwLock<HashMap<SocketAddr, Connection>>);

impl Connections {
    pub(crate) fn add(&self, conn: Connection) {
        self.0.write().insert(conn.addr, conn);
    }

    pub(crate) fn is_connected(&self, addr: SocketAddr) -> bool {
        self.0.read().contains_key(&addr)
    }

    pub(crate) fn remove(&self, addr: SocketAddr) -> Option<Connection> {
        self.0.write().remove(&addr)
    }

    pub(crate) fn num_connected(&self) -> usize {
        self.0.read().len()
    }

    pub(crate) fn addrs(&self) -> Vec<SocketAddr> {
        self.0.read().keys().copied().collect()
    }
}

/// Indicates who was the initiator and who was the responder when the connection was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionSide {
    /// The side that initiated the connection.
    Initiator,
    /// The sider that accepted the connection.
    Responder,
}

impl Not for ConnectionSide {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Initiator => Self::Responder,
            Self::Responder => Self::Initiator,
        }
    }
}

/// A helper trait to facilitate trait-objectification of connection readers.
pub(crate) trait AR: AsyncRead + Unpin + Send + Sync {}
impl<T: AsyncRead + Unpin + Send + Sync> AR for T {}

/// A helper trait to facilitate trait-objectification of connection writers.
pub(crate) trait AW: AsyncWrite + Unpin + Send + Sync {}
impl<T: AsyncWrite + Unpin + Send + Sync> AW for T {}

/// Created for each active connection; used by the protocols to obtain a handle for
/// reading and writing, and keeps track of tasks that have been spawned for the purposes
/// of the connection.
pub struct Connection {
    /// The address of the connection.
    addr: SocketAddr,
    /// The connection's side in relation to the node.
    side: ConnectionSide,
    /// Available and used only in the [`Handshake`] protocol.
    pub(crate) stream: Option<TcpStream>,
    /// Available and used only in the [`Reading`] protocol.
    pub(crate) reader: Option<Box<dyn AR>>,
    /// Available and used only in the [`Writing`] protocol.
    pub(crate) writer: Option<Box<dyn AW>>,
    /// Used to notify the [`Reading`] protocol that the connection is fully ready.
    pub(crate) readiness_notifier: Option<oneshot::Sender<()>>,
    /// Handles to tasks spawned for the connection.
    pub(crate) tasks: Vec<JoinHandle<()>>,
}

impl Connection {
    /// Creates a [`Connection`] with placeholders for protocol-related objects.
    pub(crate) fn new(addr: SocketAddr, stream: TcpStream, side: ConnectionSide) -> Self {
        Self {
            addr,
            stream: Some(stream),
            reader: None,
            writer: None,
            readiness_notifier: None,
            side,
            tasks: Default::default(),
        }
    }

    /// Returns the address associated with the connection.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// Returns `Initiator` if the associated peer initiated the connection
    /// and `Responder` if the connection request was initiated by the node.
    pub fn side(&self) -> ConnectionSide {
        self.side
    }
}
