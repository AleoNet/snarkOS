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
use crate::{protocols::Writing, Connection};

/// Can be used to automatically perform some extra actions when the node disconnects from its
/// peer, which is especially practical if the disconnect is triggered automatically, e.g. due
/// to the peer exceeding the allowed number of failures or severing its connection with the node
/// on its own.
#[async_trait::async_trait]
pub trait Disconnect: P2P
where
    Self: Clone + Send + Sync + 'static,
{
    /// Attaches the behavior specified in [`Disconnect::handle_disconnect`] to every occurrence of the
    /// node disconnecting from a peer.
    async fn enable_disconnect(&self) {
        let (from_node_sender, mut from_node_receiver) = mpsc::unbounded_channel::<(SocketAddr, oneshot::Sender<()>)>();

        // use a channel to know when the disconnect task is ready
        let (tx, rx) = oneshot::channel::<()>();

        // spawn a background task dedicated to handling disconnect events
        let self_clone = self.clone();
        let disconnect_task = tokio::spawn(async move {
            trace!(parent: self_clone.tcp().span(), "spawned the Disconnect handler task");
            tx.send(()).unwrap(); // safe; the channel was just opened

            while let Some((peer_addr, notifier)) = from_node_receiver.recv().await {
                let self_clone2 = self_clone.clone();
                tokio::spawn(async move {
                    // perform the specified extra actions
                    self_clone2.handle_disconnect(peer_addr).await;
                    // notify the node that the extra actions have concluded
                    // and that the related connection can be dropped
                    let _ = notifier.send(()); // can't really fail
                });
            }
        });
        let _ = rx.await;
        self.tcp().tasks.lock().push(disconnect_task);

        // register the Disconnect handler with the Tcp
        let hdl = Box::new(ProtocolHandler(from_node_sender));
        assert!(
            self.tcp().protocols.disconnect.set(hdl).is_ok(),
            "the Disconnect protocol was enabled more than once!"
        );
    }

    /// Any extra actions to be executed during a disconnect; in order to still be able to
    /// communicate with the peer in the usual manner (i.e. via [`Writing`]), only its [`SocketAddr`]
    /// (as opposed to the related [`Connection`] object) is provided as an argument.
    async fn handle_disconnect(&self, peer_addr: SocketAddr);
}
