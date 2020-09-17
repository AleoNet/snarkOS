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

use crate::outbound::message::{
    read::{read_header, read_message},
    Message,
    MessageHeader,
    MessageName,
};
use snarkos_errors::network::ConnectError;

use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

/// A Channel for reading and writing messages to a peer.
/// Storing two streams allows for simultaneous reading/writing.
/// Each stream is protected by an Arc + Mutex to allow for channel cloning.
#[derive(Clone, Debug)]
pub struct Channel {
    pub address: SocketAddr,
    pub reader: Arc<Mutex<TcpStream>>,
    pub writer: Arc<Mutex<TcpStream>>,
}

impl Channel {
    pub async fn new(
        address: SocketAddr,
        reader: Arc<Mutex<TcpStream>>,
        writer: Arc<Mutex<TcpStream>>,
    ) -> Result<Self, ConnectError> {
        Ok(Self {
            address,
            reader,
            writer,
        })
    }

    /// Returns a new channel with a writer only stream.
    pub async fn new_write_only(address: SocketAddr) -> Result<Self, ConnectError> {
        let stream = Arc::new(Mutex::new(TcpStream::connect(address).await?));

        Ok(Self {
            address,
            reader: stream.clone(),
            writer: stream,
        })
    }

    /// Returns a new channel with a reader only stream.
    pub fn new_read_only(reader: TcpStream) -> Result<Self, ConnectError> {
        let address = reader.peer_addr()?;
        let stream = Arc::new(Mutex::new(reader));

        Ok(Self {
            address,
            reader: stream.clone(),
            writer: stream.clone(),
        })
    }

    /// Returns a new channel with the specified address.
    pub fn update_address(&self, address: SocketAddr) -> Self {
        Self {
            address,
            reader: self.reader.clone(),
            writer: self.writer.clone(),
        }
    }

    /// Returns a new channel with the specified reader stream.
    pub fn update_reader(&self, reader: Arc<Mutex<TcpStream>>) -> Self {
        Self {
            address: self.address,
            reader,
            writer: self.writer.clone(),
        }
    }

    /// Returns a new channel with the specified address and new writer stream.
    pub async fn update_writer(&self, address: SocketAddr) -> Result<Self, ConnectError> {
        Ok(Self {
            address,
            reader: self.reader.clone(),
            writer: Arc::new(Mutex::new(TcpStream::connect(address).await?)),
        })
    }

    /// Writes a message header + message.
    pub async fn write<M: Message>(&self, message: &M) -> Result<(), ConnectError> {
        debug!("Message {:?}, Sent to {:?}", M::name().to_string(), self.address);

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

        debug!(
            "Message {:?}, Received from {:?}",
            header.name.to_string(),
            self.address
        );

        Ok((
            header.name,
            read_message(&mut *self.reader.lock().await, header.len as usize).await?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::outbound::message_types::{Ping, Pong};

    use super::*;
    use snarkos_testing::network::{random_socket_address, simulate_active_node};

    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_write() {
        // 1. Start peer node

        let peer_address = random_socket_address();
        simulate_active_node(peer_address).await;

        // 2. Server connect to peer

        let server_channel = Channel::new_write_only(peer_address).await.unwrap();

        // 3. Server write message to peer

        server_channel.write(&Ping::new()).await.unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_read() {
        let peer_address = random_socket_address();
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        tokio::spawn(async move {
            // 1. Server connects to peer

            let server_channel = Channel::new_write_only(peer_address).await.unwrap();

            // 2. Server writes ping message

            server_channel.write(&Ping::new()).await.unwrap();
        });

        // 2. Peer accepts server connection

        let (reader, _address) = peer_listener.accept().await.unwrap();
        let peer_channel = Channel::new_read_only(reader).unwrap();

        // 4. Peer reads ping message

        let (name, bytes) = peer_channel.read().await.unwrap();

        assert_eq!(Ping::name(), name);
        assert!(Ping::deserialize(bytes).is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_channel_update() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let (ty, ry) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let mut server_listener = TcpListener::bind(server_address).await.unwrap();

            tx.send(()).unwrap();

            // 1. Server connects to peer

            let mut channel = Channel::new_write_only(peer_address).await.unwrap();

            // 4. Server accepts peer connection

            let (reader, _socket) = server_listener.accept().await.unwrap();

            channel = channel.update_reader(Arc::new(Mutex::new(reader)));

            // 5. Server writes ping

            let server_ping = Ping {
                nonce: 18446744073709551615u64,
            };

            channel.write(&server_ping).await.unwrap();

            // 6. Server reads pong

            let (name, bytes) = channel.read().await.unwrap();
            let peer_pong = Pong::deserialize(bytes).unwrap();

            assert_eq!(Pong::name(), name);
            assert_eq!(Pong::new(server_ping), peer_pong);
            ty.send(()).unwrap();
        });
        rx.await.unwrap();

        // 2. Peer accepts server connection

        let (reader, _address) = peer_listener.accept().await.unwrap();
        let mut channel = Channel::new_read_only(reader).unwrap();

        // 3. Peer connects to server

        channel = channel.update_writer(server_address).await.unwrap();

        // 6. Peer reads ping message

        let (name, bytes) = channel.read().await.unwrap();
        let server_ping = Ping::deserialize(bytes).unwrap();

        assert_eq!(Ping::name(), name);
        assert_eq!(
            Ping {
                nonce: 18446744073709551615u64
            },
            server_ping
        );

        // 7. Peer writes pong message

        channel.write(&Pong::new(server_ping)).await.unwrap();

        ry.await.unwrap();
    }
}
