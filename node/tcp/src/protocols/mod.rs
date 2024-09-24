// Copyright 2024 Aleo Network Foundation
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

//! Opt-in protocols available to the node; each protocol is expected to spawn its own task that runs throughout the
//! node's lifetime and handles a specific functionality. The communication with these tasks is done via dedicated
//! handler objects.

use std::{io, net::SocketAddr};

use once_cell::race::OnceBox;
use tokio::sync::{mpsc, oneshot};

use crate::connections::Connection;

mod disconnect;
mod handshake;
mod on_connect;
mod reading;
mod writing;

pub use disconnect::Disconnect;
pub use handshake::Handshake;
pub use on_connect::OnConnect;
pub use reading::Reading;
pub use writing::Writing;

#[derive(Default)]
pub(crate) struct Protocols {
    pub(crate) handshake: OnceBox<ProtocolHandler<Connection, io::Result<Connection>>>,
    pub(crate) reading: OnceBox<ProtocolHandler<Connection, io::Result<Connection>>>,
    pub(crate) writing: OnceBox<writing::WritingHandler>,
    pub(crate) on_connect: OnceBox<ProtocolHandler<SocketAddr, ()>>,
    pub(crate) disconnect: OnceBox<ProtocolHandler<SocketAddr, ()>>,
}

/// An object sent to a protocol handler task; the task assumes control of a protocol-relevant item `T`,
/// and when it's done with it, it returns it (possibly in a wrapper object) or another relevant object
/// to the callsite via the counterpart [`oneshot::Receiver`].
pub(crate) type ReturnableItem<T, U> = (T, oneshot::Sender<U>);

pub(crate) type ReturnableConnection = ReturnableItem<Connection, io::Result<Connection>>;

pub(crate) struct ProtocolHandler<T, U>(mpsc::UnboundedSender<ReturnableItem<T, U>>);

pub(crate) trait Protocol<T, U> {
    fn trigger(&self, item: ReturnableItem<T, U>);
}

impl<T, U> Protocol<T, U> for ProtocolHandler<T, U> {
    fn trigger(&self, item: ReturnableItem<T, U>) {
        // ignore errors; they can only happen if a disconnect interrupts the protocol setup process
        let _ = self.0.send(item);
    }
}
