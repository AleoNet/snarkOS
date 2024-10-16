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
