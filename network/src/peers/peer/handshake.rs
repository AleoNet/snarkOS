// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use std::net::SocketAddr;

use snow::TransportState;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use snarkos_metrics::{self as metrics, handshakes::*};

use crate::{
    peer::{cipher::Cipher, network::PeerIOHandle},
    NetworkError, Peer, Version,
};

pub struct HandshakeData {
    pub version: Version,
    pub noise: TransportState,
    pub buffer: Box<[u8]>,
    pub noise_buffer: Box<[u8]>,
}

async fn responder_handshake<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
    remote_address: SocketAddr,
    own_version: &Version,
    writer: &mut W,
    reader: &mut R,
) -> Result<HandshakeData, NetworkError> {
    let builder = snow::Builder::with_resolver(
        crate::HANDSHAKE_PATTERN
            .parse()
            .expect("Invalid noise handshake pattern!"),
        Box::new(snow::resolvers::SodiumResolver),
    );
    let static_key = builder.generate_keypair()?.private;
    let noise_builder = builder.local_private_key(&static_key).psk(3, crate::HANDSHAKE_PSK);
    let mut noise = noise_builder.build_responder()?;
    let mut buffer: Box<[u8]> = vec![0u8; crate::MAX_MESSAGE_SIZE + 4096].into();
    let mut noise_buffer: Box<[u8]> = vec![0u8; crate::NOISE_BUF_LEN].into();
    // <- e
    reader.read_exact(&mut buffer[..1]).await?;
    let len = buffer[0] as usize;
    if len == 0 {
        return Err(NetworkError::InvalidHandshake);
    }
    let len = reader.read_exact(&mut buffer[..len]).await?;
    noise.read_message(&buffer[..len], &mut noise_buffer)?;
    trace!("received e (XX handshake part 1/3) from {}", remote_address);

    // -> e, ee, s, es
    let serialized_version = Version::serialize(&own_version).unwrap();
    let len = noise.write_message(&serialized_version, &mut noise_buffer)?;
    writer.write_all(&[len as u8]).await?;
    writer.write_all(&noise_buffer[..len]).await?;
    writer.flush().await?;
    trace!("sent e, ee, s, es (XX handshake part 2/3) to {}", remote_address);

    // <- s, se, psk
    reader.read_exact(&mut buffer[..1]).await?;
    let len = buffer[0] as usize;
    if len == 0 {
        return Err(NetworkError::InvalidHandshake);
    }
    let len = reader.read_exact(&mut buffer[..len]).await?;
    let len = noise.read_message(&buffer[..len], &mut noise_buffer)?;
    let peer_version = Version::deserialize(&noise_buffer[..len])?;
    trace!("received s, se, psk (XX handshake part 3/3) from {}", remote_address);

    if peer_version.node_id == own_version.node_id {
        return Err(NetworkError::SelfConnectAttempt);
    }
    if peer_version.version != crate::PROTOCOL_VERSION {
        return Err(NetworkError::InvalidHandshake);
    }

    metrics::increment_counter!(SUCCESSES_RESP);
    Ok(HandshakeData {
        version: peer_version,
        noise: noise.into_transport_mode()?,
        buffer,
        noise_buffer,
    })
}

async fn initiator_handshake<W: AsyncWrite + Unpin, R: AsyncRead + Unpin>(
    remote_address: SocketAddr,
    own_version: &Version,
    writer: &mut W,
    reader: &mut R,
) -> Result<HandshakeData, NetworkError> {
    let builder = snow::Builder::with_resolver(
        crate::HANDSHAKE_PATTERN
            .parse()
            .expect("Invalid noise handshake pattern!"),
        Box::new(snow::resolvers::SodiumResolver),
    );
    let static_key = builder.generate_keypair()?.private;
    let noise_builder = builder.local_private_key(&static_key).psk(3, crate::HANDSHAKE_PSK);
    let mut noise = noise_builder.build_initiator()?;
    let mut buffer: Box<[u8]> = vec![0u8; crate::MAX_MESSAGE_SIZE + 4096].into();
    let mut noise_buffer: Box<[u8]> = vec![0u8; crate::NOISE_BUF_LEN].into();
    // -> e
    let len = noise.write_message(&[], &mut buffer)?;
    writer.write_all(&[len as u8]).await?;
    writer.write_all(&buffer[..len]).await?;
    writer.flush().await?;
    trace!("sent e (XX handshake part 1/3) to {}", remote_address);

    // <- e, ee, s, es
    reader.read_exact(&mut noise_buffer[..1]).await?;
    let len = noise_buffer[0] as usize;
    if len == 0 {
        return Err(NetworkError::InvalidHandshake);
    }
    let len = reader.read_exact(&mut noise_buffer[..len]).await?;
    let len = noise.read_message(&noise_buffer[..len], &mut buffer)?;
    let version = Version::deserialize(&buffer[..len])?;
    trace!("received e, ee, s, es (XX handshake part 2/3) from {}", remote_address);

    if version.node_id == own_version.node_id {
        return Err(NetworkError::SelfConnectAttempt);
    }
    if version.version != crate::PROTOCOL_VERSION {
        return Err(NetworkError::InvalidHandshake);
    }

    // -> s, se, psk
    let own_version = Version::serialize(own_version)?;
    let len = noise.write_message(&own_version, &mut buffer)?;
    writer.write_all(&[len as u8]).await?;
    writer.write_all(&buffer[..len]).await?;
    writer.flush().await?;
    trace!("sent s, se, psk (XX handshake part 3/3) to {}", remote_address);

    metrics::increment_counter!(SUCCESSES_INIT);
    Ok(HandshakeData {
        version,
        noise: noise.into_transport_mode()?,
        buffer,
        noise_buffer,
    })
}

impl Peer {
    pub(super) async fn inner_handshake_initiator(
        &mut self,
        stream: TcpStream,
        our_version: Version,
    ) -> Result<PeerIOHandle, NetworkError> {
        let (mut reader, mut writer) = stream.into_split();

        let result = tokio::time::timeout(
            self.handshake_timeout(),
            initiator_handshake(self.address, &our_version, &mut writer, &mut reader),
        )
        .await;

        let data = match result {
            Ok(Ok(data)) => data,
            Ok(Err(e)) => {
                metrics::increment_counter!(FAILURES_INIT);
                return Err(e);
            }
            Err(_) => {
                metrics::increment_counter!(TIMEOUTS_INIT);
                return Err(NetworkError::HandshakeTimeout);
            }
        };

        match self.is_bootnode {
            true => info!("Connected to bootnode {}", self.address),
            false => info!("Connected to peer {}", self.address),
        };

        Ok(PeerIOHandle {
            reader: Some(reader),
            writer,
            cipher: Cipher::new(data.noise, data.buffer, data.noise_buffer),
        })
    }

    pub(super) async fn inner_handshake_responder(
        address: SocketAddr,
        stream: TcpStream,
        our_version: Version,
    ) -> Result<(Peer, PeerIOHandle), NetworkError> {
        let (mut reader, mut writer) = stream.into_split();

        let result = tokio::time::timeout(
            Peer::peer_handshake_timeout(),
            responder_handshake(address, &our_version, &mut writer, &mut reader),
        )
        .await;

        let data = match result {
            Ok(Ok(data)) => data,
            Ok(Err(e)) => {
                metrics::increment_counter!(FAILURES_RESP);
                return Err(e);
            }
            Err(_) => {
                metrics::increment_counter!(TIMEOUTS_RESP);
                return Err(NetworkError::HandshakeTimeout);
            }
        };

        let mut peer_address = address;
        peer_address.set_port(data.version.listening_port);
        let peer = Peer::new(peer_address, false);

        info!("Connected to peer {}", peer_address);

        let network = PeerIOHandle {
            reader: Some(reader),
            writer,
            cipher: Cipher::new(data.noise, data.buffer, data.noise_buffer),
        };
        Ok((peer, network))
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handshake() {
        let (responder, initiator) = tokio::io::duplex(8192);

        let mut bytes = vec![0u8; crate::MAX_MESSAGE_SIZE - crate::NOISE_TAG_LEN];
        rand::thread_rng().fill(&mut bytes[..]);

        tokio::spawn(async move {
            let (mut read, mut write) = tokio::io::split(responder);
            let data = responder_handshake(
                "127.0.0.1:1010".parse().unwrap(),
                &Version::new(crate::PROTOCOL_VERSION, 0, 0),
                &mut write,
                &mut read,
            )
            .await
            .unwrap();
            let mut cipher = Cipher::new(data.noise, data.buffer, data.noise_buffer);
            let bytes = cipher.read_packet_stream(&mut read).await.unwrap();
            assert_eq!(String::from_utf8_lossy(bytes).as_ref(), "test packet out");
            cipher
                .write_packet(&mut write, "test packet in".as_bytes())
                .await
                .unwrap();
        });

        let (mut read, mut write) = tokio::io::split(initiator);
        let data = initiator_handshake(
            "127.0.0.1:1020".parse().unwrap(),
            &Version::new(crate::PROTOCOL_VERSION, 0, 1),
            &mut write,
            &mut read,
        )
        .await
        .unwrap();
        let mut cipher = Cipher::new(data.noise, data.buffer, data.noise_buffer);
        cipher
            .write_packet(&mut write, "test packet out".as_bytes())
            .await
            .unwrap();
        let bytes = cipher.read_packet_stream(&mut read).await.unwrap();
        assert_eq!(String::from_utf8_lossy(bytes).as_ref(), "test packet in");
    }
}
