// Copyright (C) 2019-2023 Aleo Systems Inc.
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

//! Opt-in protocols available to the node; each protocol is expected to spawn its own task that runs throughout the
//! node's lifetime and handles a specific functionality. The communication with these tasks is done via dedicated
//! handler objects.

use std::net::SocketAddr;

use tokio::sync::{mpsc, oneshot};
use tracing::*;

use crate::{protocols::ProtocolHandler, P2P};
#[cfg(doc)]
use crate::{
    protocols::{Reading, Writing},
    Connection,
};

/// Can be used to automatically perform some initial actions once the connection with a peer is
/// fully established.
#[async_trait::async_trait]
pub trait OnConnect: P2P
where
    Self: Clone + Send + Sync + 'static,
{
    /// Attaches the behavior specified in [`OnConnect::on_connect`] right after every successful
    /// handshake.
    async fn enable_on_connect(&self) {
        let (from_node_sender, mut from_node_receiver) = mpsc::unbounded_channel::<(SocketAddr, oneshot::Sender<()>)>();

        // use a channel to know when the on_connect task is ready
        let (tx, rx) = oneshot::channel::<()>();

        // spawn a background task dedicated to executing the desired post-handshake actions
        let self_clone = self.clone();
        let on_connect_task = tokio::spawn(async move {
            trace!(parent: self_clone.tcp().span(), "spawned the OnConnect handler task");
            if tx.send(()).is_err() {
                error!(parent: self_clone.tcp().span(), "OnConnect handler creation interrupted! shutting down the node");
                self_clone.tcp().shut_down().await;
                return;
            }

            while let Some((addr, notifier)) = from_node_receiver.recv().await {
                let self_clone2 = self_clone.clone();
                tokio::spawn(async move {
                    // perform the specified initial actions
                    self_clone2.on_connect(addr).await;
                    // notify the node that the initial actions have concluded
                    let _ = notifier.send(()); // can't really fail
                });
            }
        });
        let _ = rx.await;
        self.tcp().tasks.lock().push(on_connect_task);

        // register the OnConnect handler with the Node
        let hdl = Box::new(ProtocolHandler(from_node_sender));
        assert!(self.tcp().protocols.on_connect.set(hdl).is_ok(), "the OnConnect protocol was enabled more than once!");
    }

    /// Any initial actions to be executed after the handshake is concluded; in order to be able to
    /// communicate with the peer in the usual manner (i.e. via [`Writing`]), only its [`SocketAddr`]
    /// (as opposed to the related [`Connection`] object) is provided as an argument.
    async fn on_connect(&self, addr: SocketAddr);
}
