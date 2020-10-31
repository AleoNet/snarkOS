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

use crate::external::message::MessageHeader;
use snarkos_errors::network::message::{MessageError, MessageHeaderError, StreamReadError};

use tokio::{io::AsyncRead, prelude::*};

/// Returns message bytes read from an input stream.
pub async fn read_message<T: AsyncRead + Unpin>(mut stream: &mut T, len: usize) -> Result<Vec<u8>, MessageError> {
    let mut buffer: Vec<u8> = vec![0; len];

    stream_read(&mut stream, &mut buffer).await?;

    Ok(buffer)
}

/// Returns a message header read from an input stream.
pub async fn read_header<T: AsyncRead + Unpin>(mut stream: &mut T) -> Result<MessageHeader, MessageHeaderError> {
    let mut buffer = [0u8; 16];

    stream_read(&mut stream, &mut buffer).await?;

    Ok(MessageHeader::from(buffer))
}

/// Reads bytes from an input stream to fill the buffer.
async fn stream_read<'a, T: AsyncRead + Unpin>(stream: &'a mut T, buffer: &'a mut [u8]) -> Result<(), StreamReadError> {
    stream.read_exact(buffer).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::{
        message::{message::Message, MessageHeader},
        message_types::{GetPeers, Version},
    };
    use snarkos_testing::network::random_socket_address;

    use serial_test::serial;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    #[serial]
    async fn read_multiple_headers() {
        let address = random_socket_address();
        let listener = TcpListener::bind(address).await.unwrap();

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
        let listener = TcpListener::bind(address).await.unwrap();
        let expected = Version::new(
            1u64,
            0u32,
            1u64,
            "0.0.0.0:4131".parse().unwrap(),
            "0.0.0.0:4141".parse().unwrap(),
        );
        let version = expected.clone();

        tokio::spawn(async move {
            let mut stream = TcpStream::connect(address).await.unwrap();
            stream.write_all(&version.serialize().unwrap()).await.unwrap();
        });

        let (mut stream, _socket) = listener.accept().await.unwrap();

        let buffer = read_message(&mut stream, 48usize).await.unwrap();
        let candidate = Version::deserialize(buffer).unwrap();
        assert_eq!(expected, candidate);
    }
}
