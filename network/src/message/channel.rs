use crate::message::{
    read::{read_header, read_message},
    Message,
    MessageHeader,
    MessageName,
};

use snarkos_errors::network::ConnectError;
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::Mutex};

/// Allows for reading and writing messages to a peer.
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

    /// Returns a new channel with a writer only stream
    pub async fn new_write_only(address: SocketAddr) -> Result<Self, ConnectError> {
        let stream = Arc::new(Mutex::new(TcpStream::connect(address).await?));

        Ok(Self {
            address,
            reader: stream.clone(),
            writer: stream,
        })
    }

    /// Returns a new channel with a reader only stream
    pub fn new_read_only(reader: TcpStream) -> Result<Self, ConnectError> {
        let address = reader.peer_addr()?;
        let stream = Arc::new(Mutex::new(reader));

        Ok(Self {
            address,
            reader: stream.clone(),
            writer: stream.clone(),
        })
    }

    pub fn update_address(&mut self, address: SocketAddr) {
        self.address = address;
    }

    /// Adds a reader stream to a channel that was writer only
    pub fn update_reader(&self, reader: Arc<Mutex<TcpStream>>) -> Self {
        Self {
            address: self.address,
            reader,
            writer: self.writer.clone(),
        }
    }

    /// Adds a writer stream to a channel that was reader only
    pub async fn update_writer(&self, address: SocketAddr) -> Result<Self, ConnectError> {
        Ok(Self {
            address,
            reader: self.reader.clone(),
            writer: Arc::new(Mutex::new(TcpStream::connect(address).await?)),
        })
    }

    /// Writes a message header + message
    pub async fn write<M: Message>(&self, message: &M) -> Result<(), ConnectError> {
        info!("Message {:?}, Sent to {:?}", M::name().to_string(), self.address);

        let serialized = message.serialize()?;
        let header = MessageHeader::new(M::name(), serialized.len() as u32);

        self.writer.lock().await.write_all(&header.serialize()?).await?;
        self.writer.lock().await.write_all(&serialized).await?;

        Ok(())
    }

    /// Reads a message header + message
    pub async fn read(&self) -> Result<(MessageName, Vec<u8>), ConnectError> {
        let header = read_header(&mut *self.reader.lock().await).await?;

        info!(
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
    use crate::message::types::{Ping, Pong};

    use super::*;
    use crate::test_data::random_socket_address;
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_channel_read_write() {
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

        // 3. Peer connects to server

        let mut channel = Channel::new_read_only(reader).unwrap();
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
