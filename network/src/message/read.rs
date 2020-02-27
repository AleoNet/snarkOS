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
//
//pub async fn data_is_ready(stream: &mut TcpStream) -> Result<(), MessageHeaderError> {
//    loop {
//        let buffer = &mut [0u8; 16];
//
//        if stream.peek(buffer).poll()?.is_ready() {
//            return Ok(())
//        }
//    }
//}

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

    //    use std::sync::Arc;
    //    use tokio::sync::Mutex;
    ////    use futures::Future;
    //    use futures::future::poll_fn;
    //    use std::task::Poll;
    //    use tokio_util::codec::{
    //        Framed,
    //        LinesCodec,
    //        LinesCodecError
    //    };
    //
    ////    #[derive(Clone)]
    //    pub struct TestChannel {
    //        io: Framed<TcpStream, LinesCodec>
    ////        read: Arc<Mutex<tokio::net::tcp::ReadHalf<'a>>>,
    ////        write: Arc<Mutex<tokio::net::tcp::WriteHalf<'a>>>
    //    }
    //
    //    #[tokio::test]
    //    #[serial]
    //    async fn testing() {
    //
    //        let address = random_socket_address();
    //        let mut listener = TcpListener::bind(address).await.unwrap();
    //        let message = Ping::new();
    //        let message_copy = message.clone();
    //
    //        let (tx, rx) = tokio::sync::oneshot::channel();
    //        tokio::spawn(async move {
    ////            let test = TestChannel{ io: std::sync::Arc::new(tokio::sync::Mutex::new(TcpStream::connect(address).await.unwrap())) };
    ////            let test = TestChannel {
    ////                io: Framed::new(TcpStream::connect(address).await.unwrap(), LinesCodec::new())
    ////            };
    ////            let test_ref = test.clone();
    //
    ////            let (tz, rz) = tokio::sync::oneshot::channel();
    ////            tokio::spawn(async move {
    ////                tz.send(()).unwrap();
    ////                loop {
    ////                    println!("acquiring to read");
    ////
    ////                    let mut buf = [0; 10];
    ////                    let boolean = test_ref.io.lock().await.poll_peek(buf).is_pending();
    //////                    test_ref.io.lock().await.read_exact(buf).await.unwrap();
    ////                    println!("releasing read");
    ////                }
    ////            });
    ////            rz.await.unwrap();
    //
    ////            test.io.lock().await.write_all(&message.serialize().unwrap()).await.unwrap();
    //            println!("acquiring to write");
    //            let lines = Framed::new(TcpStream::connect(address).await.unwrap(), LinesCodec::new());
    //            lines.send(String::from("fuck you"));
    ////            test.io.lock().await.write_all(&message.serialize().unwrap()).await.unwrap();
    //            println!("releasing write");
    //
    //            tx.send(()).unwrap();
    //        });
    //
    //        let (mut stream, _sock) = listener.accept().await.unwrap();
    //        let bytes = read_message(&mut stream, 8usize).await.unwrap();
    //        let actual = Ping::deserialize(bytes).unwrap();
    //
    //        rx.await.unwrap();
    //    }
}
