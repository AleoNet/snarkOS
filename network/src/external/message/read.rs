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

use crate::{errors::message::*, external::message::*};

use tokio::{io::AsyncRead, prelude::*};

/// Returns message bytes read from an input stream.
pub async fn read_payload<'a, T: AsyncRead + Unpin>(
    mut stream: &mut T,
    buffer: &'a mut [u8],
) -> Result<&'a [u8], MessageError> {
    stream_read(&mut stream, buffer).await?;

    Ok(buffer)
}

/// Returns a message header read from an input stream.
pub async fn read_header<T: AsyncRead + Unpin>(mut stream: &mut T) -> Result<MessageHeader, MessageHeaderError> {
    let mut header_arr = [0u8; 4];
    stream_read(&mut stream, &mut header_arr).await?;
    let header = MessageHeader::from(header_arr);

    if header.len as usize > MAX_MESSAGE_SIZE {
        Err(MessageHeaderError::TooBig(header.len as usize, MAX_MESSAGE_SIZE))
    } else {
        Ok(header)
    }
}

/// Reads bytes from an input stream to fill the buffer.
async fn stream_read<'a, T: AsyncRead + Unpin>(stream: &'a mut T, buffer: &'a mut [u8]) -> Result<(), StreamReadError> {
    stream.read_exact(buffer).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::message_types::Version;
    use snarkos_testing::network::random_bound_address;

    use tokio::net::TcpStream;

    #[tokio::test]
    async fn test_write_read_message() {
        let (address, listener) = random_bound_address().await;

        let expected = Payload::Version(Version::new(1, 0, 1, 4131));
        let version = expected.clone();

        tokio::spawn(async move {
            let mut stream = TcpStream::connect(address).await.unwrap();
            let payload_bytes = bincode::serialize(&version).unwrap();
            let header = MessageHeader::from(payload_bytes.len());

            stream.write_all(&header.as_bytes()).await.unwrap();
            stream.write_all(&payload_bytes).await.unwrap();
        });

        let (mut stream, _socket) = listener.accept().await.unwrap();

        let mut buffer = [0u8; 52];
        let header = read_header(&mut stream).await.unwrap();
        let payload_bytes = read_payload(&mut stream, &mut buffer[..header.len()]).await.unwrap();
        let candidate = bincode::deserialize(&payload_bytes).unwrap();

        assert_eq!(expected, candidate);
    }
}
