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

#[cfg(doc)]
use crate::{protocols::Handshake, Config};
use crate::{
    protocols::{ProtocolHandler, ReturnableConnection},
    ConnectionSide,
    Tcp,
    P2P,
};

use async_trait::async_trait;
use bytes::BytesMut;
use futures_util::StreamExt;
use std::{io, net::SocketAddr};
use tokio::{
    io::AsyncRead,
    sync::{mpsc, oneshot},
};
use tokio_util::codec::{Decoder, FramedRead};
use tracing::*;

/// Can be used to specify and enable reading, i.e. receiving inbound messages. If the [`Handshake`]
/// protocol is enabled too, it goes into force only after the handshake has been concluded.
///
/// Each inbound message is isolated by the user-supplied [`Reading::Codec`], creating a [`Reading::Message`],
/// which is immediately queued (with a [`Reading::MESSAGE_QUEUE_DEPTH`] limit) to be processed by
/// [`Reading::process_message`]. The configured fatal IO errors result in an immediate disconnect
/// (in order to e.g. avoid accidentally reading "borked" messages).
#[async_trait]
pub trait Reading: P2P
where
    Self: Clone + Send + Sync + 'static,
{
    /// The depth of per-connection queues used to process inbound messages; the greater it is, the more inbound
    /// messages the node can enqueue, but setting it to a large value can make the node more susceptible to DoS
    /// attacks.
    ///
    /// The default value is 1024.
    const MESSAGE_QUEUE_DEPTH: usize = 1024;

    /// The initial size of a per-connection buffer for reading inbound messages. Can be set to the maximum expected size
    /// of the inbound message in order to only allocate it once.
    ///
    /// The default value is 1024KiB.
    const INITIAL_BUFFER_SIZE: usize = 1024 * 1024;

    /// The final (deserialized) type of inbound messages.
    type Message: Send;

    /// The user-supplied [`Decoder`] used to interpret inbound messages.
    type Codec: Decoder<Item = Self::Message, Error = io::Error> + Send;

    /// Prepares the node to receive messages.
    async fn enable_reading(&self) {
        let (conn_sender, mut conn_receiver) = mpsc::unbounded_channel();

        // use a channel to know when the reading task is ready
        let (tx_reading, rx_reading) = oneshot::channel();

        // the main task spawning per-connection tasks reading messages from their streams
        let self_clone = self.clone();
        let reading_task = tokio::spawn(async move {
            trace!(parent: self_clone.tcp().span(), "spawned the Reading handler task");
            tx_reading.send(()).unwrap(); // safe; the channel was just opened

            // these objects are sent from `Tcp::adapt_stream`
            while let Some(returnable_conn) = conn_receiver.recv().await {
                self_clone.handle_new_connection(returnable_conn).await;
            }
        });
        let _ = rx_reading.await;
        self.tcp().tasks.lock().push(reading_task);

        // register the Reading handler with the Tcp
        let hdl = Box::new(ProtocolHandler(conn_sender));
        assert!(self.tcp().protocols.reading.set(hdl).is_ok(), "the Reading protocol was enabled more than once!");
    }

    /// Creates a [`Decoder`] used to interpret messages from the network.
    /// The `side` param indicates the connection side **from the node's perspective**.
    fn codec(&self, addr: SocketAddr, side: ConnectionSide) -> Self::Codec;

    /// Processes an inbound message. Can be used to update state, send replies etc.
    async fn process_message(&self, source: SocketAddr, message: Self::Message) -> io::Result<()>;
}

/// This trait is used to restrict access to methods that would otherwise be public in [`Reading`].
#[async_trait]
trait ReadingInternal: Reading {
    /// Applies the [`Reading`] protocol to a single connection.
    async fn handle_new_connection(&self, (conn, conn_returner): ReturnableConnection);

    /// Wraps the user-supplied [`Decoder`] ([`Reading::Codec`]) in another one used for message accounting.
    fn map_codec<T: AsyncRead>(
        &self,
        framed: FramedRead<T, Self::Codec>,
        addr: SocketAddr,
    ) -> FramedRead<T, CountingCodec<Self::Codec>>;
}

#[async_trait]
impl<R: Reading> ReadingInternal for R {
    async fn handle_new_connection(&self, (mut conn, conn_returner): ReturnableConnection) {
        let addr = conn.addr();
        let codec = self.codec(addr, !conn.side());
        let reader = conn.reader.take().expect("missing connection reader!");
        let framed = FramedRead::new(reader, codec);
        let mut framed = self.map_codec(framed, addr);

        // the connection will notify the reading task once it's fully ready
        let (tx_conn_ready, rx_conn_ready) = oneshot::channel();
        conn.readiness_notifier = Some(tx_conn_ready);

        if Self::INITIAL_BUFFER_SIZE != 0 {
            framed.read_buffer_mut().reserve(Self::INITIAL_BUFFER_SIZE);
        }

        let (inbound_message_sender, mut inbound_message_receiver) = mpsc::channel(Self::MESSAGE_QUEUE_DEPTH);

        // use a channel to know when the processing task is ready
        let (tx_processing, rx_processing) = oneshot::channel::<()>();

        // the task for processing parsed messages
        let self_clone = self.clone();
        let inbound_processing_task = tokio::spawn(async move {
            let node = self_clone.tcp();
            trace!(parent: node.span(), "spawned a task for processing messages from {addr}");
            tx_processing.send(()).unwrap(); // safe; the channel was just opened

            while let Some(msg) = inbound_message_receiver.recv().await {
                if let Err(e) = self_clone.process_message(addr, msg).await {
                    error!(parent: node.span(), "can't process a message from {addr}: {e}");
                    node.known_peers().register_failure(addr);
                }
                #[cfg(feature = "metrics")]
                metrics::decrement_gauge(metrics::tcp::TCP_TASKS, 1f64);
            }
        });
        let _ = rx_processing.await;
        conn.tasks.push(inbound_processing_task);

        // use a channel to know when the reader task is ready
        let (tx_reader, rx_reader) = oneshot::channel::<()>();

        // the task for reading messages from a stream
        let node = self.tcp().clone();
        let reader_task = tokio::spawn(async move {
            trace!(parent: node.span(), "spawned a task for reading messages from {addr}");
            tx_reader.send(()).unwrap(); // safe; the channel was just opened

            // postpone reads until the connection is fully established; if the process fails,
            // this task gets aborted, so there is no need for a dedicated timeout
            let _ = rx_conn_ready.await;

            while let Some(bytes) = framed.next().await {
                match bytes {
                    Ok(msg) => {
                        // send the message for further processing
                        if let Err(e) = inbound_message_sender.try_send(msg) {
                            error!(parent: node.span(), "can't process a message from {addr}: {e}");
                            node.stats().register_failure();
                        }
                        #[cfg(feature = "metrics")]
                        metrics::increment_gauge(metrics::tcp::TCP_TASKS, 1f64);
                    }
                    Err(e) => {
                        error!(parent: node.span(), "can't read from {addr}: {e}");
                        node.known_peers().register_failure(addr);
                        if node.config().fatal_io_errors.contains(&e.kind()) {
                            break;
                        }
                    }
                }
            }

            let _ = node.disconnect(addr).await;
        });
        let _ = rx_reader.await;
        conn.tasks.push(reader_task);

        // return the Connection to the Tcp, resuming Tcp::adapt_stream
        if conn_returner.send(Ok(conn)).is_err() {
            unreachable!("couldn't return a Connection to the Tcp");
        }
    }

    fn map_codec<T: AsyncRead>(
        &self,
        framed: FramedRead<T, Self::Codec>,
        addr: SocketAddr,
    ) -> FramedRead<T, CountingCodec<Self::Codec>> {
        framed.map_decoder(|codec| CountingCodec { codec, node: self.tcp().clone(), addr, acc: 0 })
    }
}

/// A wrapper [`Decoder`] that also counts the inbound messages.
struct CountingCodec<D: Decoder> {
    codec: D,
    node: Tcp,
    addr: SocketAddr,
    acc: usize,
}

impl<D: Decoder> Decoder for CountingCodec<D> {
    type Error = D::Error;
    type Item = D::Item;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let initial_buf_len = src.len();
        let ret = self.codec.decode(src)?;
        let final_buf_len = src.len();
        let read_len = initial_buf_len - final_buf_len + self.acc;

        if read_len != 0 {
            trace!(parent: self.node.span(), "read {}B from {}", read_len, self.addr);

            if ret.is_some() {
                self.acc = 0;
                self.node.known_peers().register_received_message(self.addr, read_len);
                self.node.stats().register_received_message(read_len);
            } else {
                self.acc = read_len;
            }
        }

        Ok(ret)
    }
}
