use crate::message::{
    read::{read_header, read_message},
    Message,
    MessageHeader,
    MessageName,
};

use snarkos_errors::network::ConnectError;
use std::{net::SocketAddr, sync::Arc};
use tokio::{io::AsyncWriteExt, net::TcpStream, sync::RwLock};

#[derive(Clone, Debug)]
pub struct Channel {
    pub address: SocketAddr,
    pub io: Arc<RwLock<TcpStream>>,
    //    pub read:
}

impl Channel {
    pub async fn new(stream: TcpStream, address: SocketAddr) -> Result<Self, ConnectError> {
        Ok(Self {
            address,
            io: Arc::new(RwLock::new(stream)),
        })
    }

    pub async fn connect(address: SocketAddr) -> Result<Self, ConnectError> {
        // Open an asynchronous Tokio TcpStream to the socket address.
        Ok(Self {
            address,
            io: Arc::new(RwLock::new(TcpStream::connect(address).await?)),
        })
    }

    pub async fn write<M: Message>(&self, message: &M) -> Result<(), ConnectError> {
        info!("Message {:?}, Sent to {:?}", M::name().to_string(), self.address);

        let serialized = message.serialize()?;
        let header = MessageHeader::new(M::name(), serialized.len() as u32);

        self.io.write().await.write_all(&header.serialize()?).await?;
        self.io.write().await.write_all(&serialized).await?;

        Ok(())
    }

    pub async fn read(&self) -> Result<(MessageName, Vec<u8>), ConnectError> {
        let header = read_header(&mut *self.io.write().await).await?;

        info!(
            "Message {:?}, Received from {:?}",
            header.name.to_string(),
            self.address
        );

        Ok((
            header.name,
            read_message(&mut *self.io.write().await, header.len as usize).await?,
        ))
    }

    pub fn update_address(&self, address: SocketAddr) -> Result<Self, ConnectError> {
        Ok(Self {
            address,
            io: self.io.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::message::types::Ping;

    use super::*;
    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_channel_multiple_messages() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let mut listener = TcpListener::bind(address).await.unwrap();

        tokio::spawn(async move {
            let message = Ping {
                nonce: 18446744073709551615u64,
            };
            let channel = Channel::connect(address).await.unwrap();
            channel.write(&message).await.unwrap();
            channel.write(&message).await.unwrap();
        });

        let (stream, address) = listener.accept().await.unwrap();

        let channel = Channel::new(stream, address).await.unwrap();
        let (name, bytes) = channel.read().await.unwrap();

        assert_eq!(Ping::name(), name);
        assert_eq!(
            Ping {
                nonce: 18446744073709551615u64
            },
            Ping::deserialize(bytes).unwrap()
        );

        let (name, bytes) = channel.read().await.unwrap();

        assert_eq!(Ping::name(), name);
        assert_eq!(
            Ping {
                nonce: 18446744073709551615u64
            },
            Ping::deserialize(bytes).unwrap()
        );
    }
}
