#![warn(rust_2018_idioms)]

use crate::base::Message;
use snarkos_errors::network::PeerError;

use bincode;
use std::net::SocketAddr;
use tokio::{io::AsyncWriteExt, net::TcpStream};
//use chrono::Utc;
//use crate::AddressBook;
//use tokio::sync::RwLock;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Peer {
    pub address: SocketAddr,
}

impl Peer {
    pub fn new(address: SocketAddr) -> Self {
        Peer { address }
    }

    pub async fn send(self, message: &Message) -> Result<(), PeerError> {
        let mut stream = TcpStream::connect(self.address).await?;
        stream.write(&bincode::serialize(message)?).await?;

        Ok(())
    }
}

///// A network peer we can send messages to
//pub struct Peer {
//    pub address: SocketAddr,
//    pub stream: RwLock<TcpStream>,
//}
//
//impl Peer {
//    pub async fn new(address: SocketAddr) -> Result<Self, PeerError> {
//        // Open an asynchronous Tokio TcpStream to the socket address.
//        let stream =  RwLock::new(TcpStream::connect(address).await?);
//
//        Ok(Peer { address, stream })
//    }
//
//    /// Send a message to this peer by writing to its TcpStream
//    pub async fn send(self, message: &Message) -> Result<(), PeerError> {
//        self.stream
//            .write()
//            .await
//            .write(&bincode::serialize(message)?)
//            .await
//            .unwrap();
//        Ok(())
//    }
//}
//
//
//#[cfg(test)]
//mod tests {
//    use super::*;
//    use tokio::net::TcpListener;
//
//    #[tokio::test]
//    async fn test() {
//        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
//        let mut listener = TcpListener::bind(address).await.unwrap();
//
//        tokio::spawn(async move {
//            let peer = Peer::new(address).await.unwrap();
//            let message = Message::Reject;
//            peer.send(&message).await.unwrap();
//        });
//
//        listener.accept().await.unwrap();
//
//    }
//}
