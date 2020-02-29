use snarkos_errors::network::message::{MessageError, MessageHeaderError, StreamReadError};

use crate::message::MessageHeader;
use tokio::{io::AsyncRead, prelude::*};

pub async fn read_message<T: AsyncRead + Unpin>(mut stream: &mut T, len: usize) -> Result<Vec<u8>, MessageError> {
    let mut buffer: Vec<u8> = vec![0; len];

    stream_read(&mut stream, &mut buffer).await?;

    Ok(buffer)
}

pub async fn read_header<T: AsyncRead + Unpin>(mut stream: &mut T) -> Result<MessageHeader, MessageHeaderError> {
    let mut buffer = [0u8; 16];

    stream_read(&mut stream, &mut buffer).await?;

    Ok(MessageHeader::from(buffer))
}

pub async fn stream_read<'a, T: AsyncRead + Unpin>(
    stream: &'a mut T,
    buffer: &'a mut [u8],
) -> Result<usize, StreamReadError> {
    return Ok(stream.read_exact(buffer).await?);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        message::{message::Message, types::Ping, MessageHeader},
        test_data::random_socket_address,
    };
    use serial_test::serial;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    #[serial]
    async fn read_multiple_headers() {
        let address = random_socket_address();
        let mut listener = TcpListener::bind(address).await.unwrap();

        tokio::spawn(async move {
            let header = MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]);
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream.write_all(&header.serialize().unwrap()).await.unwrap();
            let header = MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8]);
            stream.write_all(&header.serialize().unwrap()).await.unwrap();
        });

        let (mut stream, _socket) = listener.accept().await.unwrap();
        let mut buf = [0u8; 16];
        stream_read(&mut stream, &mut buf).await.unwrap();

        assert_eq!(
            MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]),
            MessageHeader::from(buf)
        );

        let mut buf = [0u8; 16];
        stream_read(&mut stream, &mut buf).await.unwrap();

        assert_eq!(
            MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8]),
            MessageHeader::from(buf)
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_read_header() {
        let address = random_socket_address();
        let mut listener = TcpListener::bind(address).await.unwrap();

        tokio::spawn(async move {
            let header = MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]);
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream.write_all(&header.serialize().unwrap()).await.unwrap();
        });

        let (mut stream, _socket) = listener.accept().await.unwrap();

        let header = read_header(&mut stream).await.unwrap();

        assert_eq!(
            MessageHeader::from([112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4]),
            header
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_read_message() {
        let address = random_socket_address();
        let mut listener = TcpListener::bind(address).await.unwrap();
        let message = Ping::new();
        let message_copy = message.clone();

        tokio::spawn(async move {
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream.write_all(&message.serialize().unwrap()).await.unwrap();
        });

        let (mut stream, _socket) = listener.accept().await.unwrap();

        let bytes = read_message(&mut stream, 8usize).await.unwrap();
        let actual = Ping::deserialize(bytes).unwrap();

        assert_eq!(message_copy, actual);
    }

    #[tokio::test]
    #[serial]
    async fn test_two_streams() {
        let server_address = random_socket_address();
        let peer_address = random_socket_address();

        // peer listens
        let mut peer_listener = TcpListener::bind(peer_address).await.unwrap();

        let server_ping = Ping::new();
        let peer_ping = Ping::new();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let (tz, rz) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            // server listens
            let mut server_listener = TcpListener::bind(server_address).await.unwrap();

            // server connects
            println!("server connecting...");
            let mut server_write_stream = TcpStream::connect(peer_address).await.unwrap();

            tz.send(()).unwrap();

            // server accepts
            println!("server accepting...");
            let (mut server_read_stream, _sock) = server_listener.accept().await.unwrap();

            let (ty, ry) = tokio::sync::oneshot::channel();
            tokio::spawn(async move {
                println!("server reader thread started");
                ty.send(()).unwrap();

                let bytes = read_message(&mut server_read_stream, 8usize).await.unwrap();
                let message = Ping::deserialize(bytes).unwrap();
                println!("server reader read    {:?}", message);
                tx.send(()).unwrap();
            });

            println!("server writer starting");

            ry.await.unwrap();

            println!("server writer writing {:?}", server_ping);

            server_write_stream
                .write_all(&server_ping.serialize().unwrap())
                .await
                .unwrap();
        });

        // peer accepts
        println!("peer accepting...");
        let (mut peer_read_stream, _sock) = peer_listener.accept().await.unwrap();

        rz.await.unwrap();

        // peer connects
        println!("peer connecting...");
        let mut peer_write_stream = TcpStream::connect(server_address).await.unwrap();

        println!("peer reading from writer thread");

        let bytes = read_message(&mut peer_read_stream, 8usize).await.unwrap();
        let message = Ping::deserialize(bytes).unwrap();

        println!("peer read             {:?}", message);

        println!("peer writing          {:?}", peer_ping);
        peer_write_stream
            .write_all(&peer_ping.serialize().unwrap())
            .await
            .unwrap();

        rx.await.unwrap();
    }
}
