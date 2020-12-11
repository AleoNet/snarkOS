// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use crate::external::message::{
    read::{read_header, read_message},
    Message,
    MessageHeader,
    MessageName,
};
use snarkos_errors::network::ConnectError;

use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::Mutex,
};

/// A channel for reading and writing messages to a peer.
/// The channel manages two streams to allow for simultaneous reading and writing.
/// Each stream is protected by an Arc + Mutex to allow for channel cloning.
#[derive(Clone, Debug)]
pub struct Channel {
    pub remote_address: SocketAddr,
    pub reader: Arc<Mutex<OwnedReadHalf>>,
    pub writer: Arc<Mutex<OwnedWriteHalf>>,
}

impl Channel {
    pub fn new(remote_address: SocketAddr, stream: TcpStream) -> Result<Self, ConnectError> {
        let (reader, writer) = stream.into_split();

        Ok(Self {
            remote_address,
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }

    pub async fn from_addr(remote_address: SocketAddr) -> Result<Self, ConnectError> {
        let stream = TcpStream::connect(remote_address).await?;

        Channel::new(remote_address, stream)
    }

    /// Returns a new channel with the specified address and new writer stream.
    pub async fn update_writer(self, remote_address: SocketAddr) -> Result<Self, ConnectError> {
        let stream = TcpStream::connect(remote_address).await?;
        let writer = stream.into_split().1;

        Ok(Self {
            remote_address,
            reader: self.reader,
            writer: Arc::new(Mutex::new(writer)),
        })
    }

    pub async fn update_reader(self, stream: TcpStream) -> Result<Self, ConnectError> {
        let reader = stream.into_split().0;

        Ok(Self {
            remote_address: self.remote_address,
            reader: Arc::new(Mutex::new(reader)),
            writer: self.writer,
        })
    }

    /// Writes a message header + message.
    pub async fn write<M: Message>(&self, message: &M) -> Result<(), ConnectError> {
        debug!("Send {:?} message to {:?}", M::name().to_string(), self.remote_address);

        let serialized = message.serialize()?;
        let header = MessageHeader::new(M::name(), serialized.len() as u32);

        let mut writer = self.writer.lock().await;
        writer.write_all(&header.serialize()?).await?;
        writer.write_all(&serialized).await?;

        Ok(())
    }

    /// Reads a message header + message.
    pub async fn read(&self) -> Result<(MessageName, Vec<u8>), ConnectError> {
        let header = read_header(&mut *self.reader.lock().await).await?;

        debug!("Received `{}` request from {}", header.name, self.remote_address);

        Ok((
            header.name,
            read_message(&mut *self.reader.lock().await, header.len as usize).await?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::message_types::{GetPeers, Peers};
    use snarkos_testing::network::{random_bound_address, simulate_active_node};

    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn channel_write() {
        // 1. Start remote node
        let remote_address = simulate_active_node().await;

        // 2. Server connect to peer
        let server_channel = Channel::from_addr(remote_address).await.unwrap();

        // 3. Server write message to peer
        server_channel.write(&GetPeers).await.unwrap();
    }

    #[tokio::test]
    async fn channel_read() {
        let (remote_address, remote_listener) = random_bound_address().await;

        tokio::spawn(async move {
            // 1. Server connects to peer.
            let server_channel = Channel::from_addr(remote_address).await.unwrap();

            // 2. Server writes GetPeers message.
            server_channel.write(&GetPeers).await.unwrap();
        });

        // 2. Peer accepts server connection.
        let (stream, address) = remote_listener.accept().await.unwrap();
        let peer_channel = Channel::new(address, stream).unwrap();

        // 4. Peer reads GetPeers message.
        let (name, buffer) = peer_channel.read().await.unwrap();

        assert_eq!(GetPeers::name(), name);
        assert!(GetPeers::deserialize(buffer).is_ok());
    }

    #[tokio::test]
    async fn channel_update() {
        // start a listener for the "remote" node
        let (remote_address, remote_listener) = random_bound_address().await;

        let (tx, rx) = tokio::sync::oneshot::channel();
        let (ty, ry) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            // start a listener for the "local" node
            let (local_address, server_listener) = random_bound_address().await;

            tx.send(local_address).unwrap();

            // 1. Local node connects to Remote node
            let mut channel = Channel::from_addr(remote_address).await.unwrap();

            // 4. Local node accepts Remote node connection
            let (stream, address) = server_listener.accept().await.unwrap();
            channel = channel.update_reader(stream).await.unwrap();

            // 5. Local node writes GetPeers message
            channel.write(&GetPeers).await.unwrap();

            // 6. Local node reads Peers message
            let (name, buffer) = channel.read().await.unwrap();
            assert_eq!(Peers::name(), name);
            assert_eq!(Peers::new(vec![]), Peers::deserialize(buffer).unwrap());

            ty.send(()).unwrap();
        });

        let local_address = rx.await.unwrap();

        // 2. Remote node accepts Local node connection.
        let (stream, _) = remote_listener.accept().await.unwrap();
        let mut channel = Channel::new(local_address, stream).unwrap();

        // 3. Remote node connects to Local node.
        channel = channel.update_writer(local_address).await.unwrap();

        // 6. Remote node reads GetPeers message.
        let (name, buffer) = channel.read().await.unwrap();
        assert_eq!(GetPeers::name(), name);
        assert_eq!(GetPeers, GetPeers::deserialize(buffer).unwrap());

        // 7. Remote node writes Peers message.
        channel.write(&Peers::new(vec![])).await.unwrap();

        ry.await.unwrap();
    }
}
