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

use crate::{
    helpers::BlockCache,
    network::{
        handshake::{cipher::Cipher, initiator_handshake, responder_handshake},
        message::Message,
        peer_quality::{PeerQuality, SyncState},
        NetworkError,
        Version,
    },
    Environment,
    Node,
};
use snarkvm::dpc::Network;

use anyhow::{anyhow, Result};
use mpmc_map::MpmcMap;
use std::{
    io::{Error as IoError, ErrorKind},
    marker::PhantomData,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, AtomicU32},
        Arc,
    },
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    sync::mpsc,
    time::timeout,
};

pub(super) enum PeerAction<N: Network> {
    Disconnect,
    Send(Message<N>, Option<Instant>),
    // Get(oneshot::Sender<Peer<N, E>>),
    QualityJudgement,
    CancelSync,
    GotSyncBlock,
    ExpectingSyncBlocks(u32),
    SoftFail,
}

pub struct PeerHandler<N: Network, E: Environment<N>> {
    reader: Option<OwnedReadHalf>,
    writer: OwnedWriteHalf,
    cipher: Cipher<N, E>,
}

impl<N: Network, E: Environment<N>> PeerHandler<N, E> {
    pub async fn write_payload(&mut self, message: &Message<N>) -> Result<()> {
        let serialized_message = bincode::serialize(message)?;
        self.cipher.write_packet(&mut self.writer, &serialized_message[..]).await?;
        Ok(())
    }

    pub fn read_payload(&mut self, message: &[u8]) -> Result<&[u8]> {
        Ok(self.cipher.read_packet(message)?)
    }

    pub fn take_reader(&mut self) -> PeerReader<N, E, OwnedReadHalf> {
        PeerReader {
            reader: self.reader.take().unwrap(),
            buffer: vec![0u8; E::NOISE_BUFFER_LENGTH],
            _phantom: PhantomData,
        }
    }
}

pub struct PeerReader<N: Network, E: Environment<N>, R: AsyncRead + Unpin + 'static> {
    pub reader: R,
    pub buffer: Vec<u8>,
    _phantom: PhantomData<(N, E)>,
}

impl<N: Network, E: Environment<N>, R: AsyncRead + Unpin + 'static> PeerReader<N, E, R> {
    pub async fn read_raw_payload(&mut self) -> Result<&[u8]> {
        let length = self.reader.read_u32().await? as usize;
        if length > E::MAX_MESSAGE_SIZE {
            return Err(NetworkError::MessageTooBig(length).into());
        } else if length == 0 {
            return Err(NetworkError::ZeroLengthMessage.into());
        }

        if self.buffer.len() < length {
            self.buffer.resize(length, 0);
        }

        self.reader.read_exact(&mut self.buffer[..length]).await?;
        Ok(&self.buffer[..length])
    }
}

/// A data structure containing information about a peer.
#[derive(Clone, Debug)]
pub struct Peer<N: Network, E: Environment<N>> {
    /// The IP address of the peer.
    ip: SocketAddr,
    /// The latest broadcast block height of the peer.
    block_height: u32,
    /// Quantifies the connection quality with the peer.
    quality: PeerQuality<N, E>,
    /// Tracks the sync state with the peer.
    sync_state: SyncState,

    /// The cache of received blocks from the peer.
    pub block_received_cache: BlockCache,
}

impl<N: Network, E: Environment<N>> Peer<N, E> {
    pub(crate) fn new(ip: SocketAddr) -> Self {
        Self {
            ip,
            block_height: 0,
            quality: Default::default(),
            sync_state: Default::default(),
            block_received_cache: BlockCache::default(),
        }
    }

    pub fn connect(mut self, node: Node<N, E>) {
        let (sender, receiver) = mpsc::channel(64);

        tokio::spawn(async move {
            self.set_connecting();

            // Initiate a connection request to the IP address of the peer.
            let stream = match timeout(Duration::from_secs(E::CONNECTION_TIMEOUT_SECS), TcpStream::connect(self.ip)).await {
                Ok(stream) => match stream {
                    Ok(stream) => stream,
                    Err(error) => {
                        self.set_connecting_failed();
                        return Err(anyhow!("Failed to send outgoing connection to '{}': '{:?}'", self.ip, error));
                    }
                },
                Err(_) => {
                    self.set_connecting_failed();
                    return Err(IoError::new(ErrorKind::TimedOut, "connection timed out").into());
                }
            };

            // On initial success, split the stream into a reader and writer for communicating.
            let (mut reader, mut writer) = stream.into_split();

            // Initiate the handshake sequence.
            let handshake = match timeout(
                Duration::from_secs(E::HANDSHAKE_TIMEOUT_SECS),
                initiator_handshake::<N, E, _, _>(self.ip, &node.version(), &mut writer, &mut reader),
            )
            .await
            {
                Ok(Ok(handshake)) => handshake,
                Ok(Err(error)) => {
                    self.set_connecting_failed();
                    return Err(error);
                }
                Err(error) => {
                    error!("Failed to send outgoing connection to '{}': '{:?}'", self.ip, error);
                    self.set_connecting_failed();
                    return Err(NetworkError::HandshakeTimeout.into());
                }
            };

            info!("Connected to {}", self.ip);

            let handler = PeerHandler {
                reader: Some(reader),
                writer,
                cipher: Cipher::new(handshake.noise, handshake.buffer, handshake.noise_buffer),
            };

            self.set_connected();

            if let Err(error) = self.run(node, handler, receiver).await {
                if !error.is_trivial() {
                    self.set_fail();
                    error!("Unrecoverable failure communicating to outbound peer '{}': '{:?}'", self.ip, error);
                } else {
                    warn!("Unrecoverable failure communicating to outbound peer '{}': '{:?}'", self.ip, error);
                }
            }

            self.set_disconnected();
            Ok(())
        });
    }

    pub fn receive(remote_address: SocketAddr, node: Node<N, E>, stream: TcpStream) {
        let (sender, receiver) = mpsc::channel(64);

        tokio::spawn(async move {
            let (peer, network) = match Peer::handle_receive(remote_address, stream, node.version()).await {
                Ok((peer, network)) => (peer, network),
                Err(error) => {
                    error!("Failed to receive incoming connection from '{}': '{:?}'", remote_address, error);
                    return;
                }
            };

            let mut peer = node.peers.fetch_received_peer_data(peer.ip).await;
            peer.set_connected();

            if let Err(error) = peer.run(node, network, receiver).await {
                if !error.is_trivial() {
                    peer.set_fail();
                    error!("Unrecoverable failure communicating to inbound peer '{}': '{:?}'", peer.ip, error);
                } else {
                    warn!("Unrecoverable failure communicating to inbound peer '{}': '{:?}'", peer.ip, error);
                }
            }

            peer.set_disconnected();
        });
    }

    async fn handle_sender(&mut self, stream: TcpStream, our_version: Version) -> Result<PeerHandler<N, E>> {
        let (mut reader, mut writer) = stream.into_split();

        let result = timeout(
            Duration::from_secs(E::HANDSHAKE_TIMEOUT_SECS),
            initiator_handshake::<N, E, _, _>(self.ip, &our_version, &mut writer, &mut reader),
        )
        .await;

        let handshake = match result {
            Ok(Ok(handshake)) => handshake,
            Ok(Err(error)) => return Err(error),
            Err(_) => return Err(NetworkError::HandshakeTimeout.into()),
        };

        info!("Connected to peer {}", self.ip);

        Ok(PeerHandler {
            reader: Some(reader),
            writer,
            cipher: Cipher::new(handshake.noise, handshake.buffer, handshake.noise_buffer),
        })
    }

    async fn handle_receive(
        remote_address: SocketAddr,
        stream: TcpStream,
        our_version: Version,
    ) -> Result<(Peer<N, E>, PeerHandler<N, E>)> {
        let (mut reader, mut writer) = stream.into_split();

        let handshake_timeout = Duration::from_secs(E::HANDSHAKE_TIMEOUT_SECS);
        let result = timeout(
            handshake_timeout,
            responder_handshake::<N, E, _, _>(remote_address, &our_version, &mut writer, &mut reader),
        )
        .await;

        let handshake = match result {
            Ok(Ok(handshake)) => handshake,
            Ok(Err(error)) => return Err(error.into()),
            Err(_) => return Err(NetworkError::HandshakeTimeout.into()),
        };

        let mut ip = remote_address;
        ip.set_port(handshake.version.listening_port);

        info!("Connected to peer {}", ip);

        Ok((
            Self {
                ip,
                block_height: 0,
                quality: Default::default(),
                sync_state: Default::default(),
                block_received_cache: BlockCache::default(),
            },
            PeerHandler {
                reader: Some(reader),
                writer,
                cipher: Cipher::new(handshake.noise, handshake.buffer, handshake.noise_buffer),
            },
        ))
    }

    async fn run(
        &mut self,
        node: Node<N, E>,
        mut network: PeerHandler<N, E>,
        mut receiver: mpsc::Receiver<PeerAction<N>>,
    ) -> Result<(), NetworkError> {
        let mut reader = network.take_reader();

        let (sender, mut read_receiver) = mpsc::channel(8);

        tokio::spawn(async move {
            loop {
                if sender.send(reader.read_raw_payload().await.map(|x| x.to_vec())).await.is_err() {
                    break;
                }
            }
        });

        loop {
            tokio::select! {
                        biased;

                        message = receiver.recv() => {
            //                 if message.is_none() {
            //                     break;
            //                 }
            //                 let message = message.unwrap();
            //                 match self.process_message(&mut network, message).await? {
            //                     PeerResponse::Disconnect => break,
            //                     PeerResponse::None => (),
            //                 }
                        },
                        data = read_receiver.recv() => {
            //                 if data.is_none() {
            //                     break;
            //                 }
            //
            //                 let data = match data.unwrap() {
            //                     // decrypt
            //                     Ok(data) => network.read_payload(&data[..]),
            //                     Err(e) => Err(e)
            //                 };
            //
            //                 let deserialized = self.deserialize_payload(data);
            //
            //                 let time_received = match deserialized {
            //                     Ok(Payload::GetPeers)
            //                     | Ok(Payload::GetSync(_))
            //                     | Ok(Payload::GetBlocks(_))
            //                     | Ok(Payload::GetMemoryPool) => Some(Instant::now()),
            //                     _ => None,
            //                 };
            //
            //                 self.dispatch_payload(&node, &mut network, time_received, deserialized).await?;
                        },
                    }
        }

        Ok(())
    }

    fn set_connecting(&mut self) {
        self.quality.connecting();
    }

    fn set_connecting_failed(&mut self) {
        self.quality.connect_failed();
        self.set_fail();
    }

    fn set_connected(&mut self) {
        self.quality.connected();
    }

    fn set_disconnected(&mut self) {
        self.sync_state.reset();
        self.quality.disconnected();
    }

    fn set_fail(&mut self) {
        self.quality.set_fail();
    }

    pub(super) fn failures(&mut self) -> usize {
        self.quality.failures()
    }
}
