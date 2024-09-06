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

use std::{io, time::Duration};

use tokio::{
    io::{split, AsyncRead, AsyncWrite},
    net::TcpStream,
    sync::{mpsc, oneshot},
    time::timeout,
};
use tracing::*;

use crate::{
    protocols::{ProtocolHandler, ReturnableConnection},
    Connection,
    P2P,
};

/// Can be used to specify and enable network handshakes. Upon establishing a connection, both sides will
/// need to adhere to the specified handshake rules in order to finalize the connection and be able to send
/// or receive any messages.
#[async_trait::async_trait]
pub trait Handshake: P2P
where
    Self: Clone + Send + Sync + 'static,
{
    /// The maximum time allowed for a connection to perform a handshake before it is rejected.
    ///
    /// The default value is 3000ms.
    const TIMEOUT_MS: u64 = 3_000;

    /// Prepares the node to perform specified network handshakes.
    async fn enable_handshake(&self) {
        let (from_node_sender, mut from_node_receiver) = mpsc::unbounded_channel::<ReturnableConnection>();

        // use a channel to know when the handshake task is ready
        let (tx, rx) = oneshot::channel();

        // spawn a background task dedicated to handling the handshakes
        let self_clone = self.clone();
        let handshake_task = tokio::spawn(async move {
            trace!(parent: self_clone.tcp().span(), "spawned the Handshake handler task");
            tx.send(()).unwrap(); // safe; the channel was just opened

            while let Some((conn, result_sender)) = from_node_receiver.recv().await {
                let addr = conn.addr();

                let node = self_clone.clone();
                tokio::spawn(async move {
                    debug!(parent: node.tcp().span(), "shaking hands with {} as the {:?}", addr, !conn.side());
                    let result = timeout(Duration::from_millis(Self::TIMEOUT_MS), node.perform_handshake(conn)).await;

                    let ret = match result {
                        Ok(Ok(conn)) => {
                            debug!(parent: node.tcp().span(), "successfully handshaken with {}", addr);
                            Ok(conn)
                        }
                        Ok(Err(e)) => {
                            error!(parent: node.tcp().span(), "handshake with {} failed: {}", addr, e);
                            Err(e)
                        }
                        Err(_) => {
                            error!(parent: node.tcp().span(), "handshake with {} timed out", addr);
                            Err(io::ErrorKind::TimedOut.into())
                        }
                    };

                    // return the Connection to the Tcp, resuming Tcp::adapt_stream
                    if result_sender.send(ret).is_err() {
                        unreachable!("couldn't return a Connection to the Tcp");
                    }
                });
            }
        });
        let _ = rx.await;
        self.tcp().tasks.lock().push(handshake_task);

        // register the Handshake handler with the Tcp
        let hdl = Box::new(ProtocolHandler(from_node_sender));
        assert!(self.tcp().protocols.handshake.set(hdl).is_ok(), "the Handshake protocol was enabled more than once!");
    }

    /// Performs the handshake; temporarily assumes control of the [`Connection`] and returns it if the handshake is
    /// successful.
    async fn perform_handshake(&self, conn: Connection) -> io::Result<Connection>;

    /// Borrows the full connection stream to be used in the implementation of [`Handshake::perform_handshake`].
    fn borrow_stream<'a>(&self, conn: &'a mut Connection) -> &'a mut TcpStream {
        conn.stream.as_mut().unwrap()
    }

    /// Assumes full control of a connection's stream in the implementation of [`Handshake::perform_handshake`], by
    /// the end of which it *must* be followed by [`Handshake::return_stream`].
    fn take_stream(&self, conn: &mut Connection) -> TcpStream {
        conn.stream.take().unwrap()
    }

    /// This method only needs to be called if [`Handshake::take_stream`] had been called before; it is used to
    /// return a (potentially modified) stream back to the applicable connection.
    fn return_stream<T: AsyncRead + AsyncWrite + Send + Sync + 'static>(&self, conn: &mut Connection, stream: T) {
        let (reader, writer) = split(stream);
        conn.reader = Some(Box::new(reader));
        conn.writer = Some(Box::new(writer));
    }
}
