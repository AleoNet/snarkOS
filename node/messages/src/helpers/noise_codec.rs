// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use std::{io, sync::Arc};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use kadmium::{codec::MessageCodec as KadmiumCodec, message::Message as KadmiumMessage};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use snarkvm::prelude::Testnet3;
use snow::{HandshakeState, StatelessTransportState};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

use crate::{Message, MessageCodec};

type CurrentNetwork = Testnet3;

/// The maximum message size for noise messages. If the data to be encrypted exceeds it, it is chunked.
const MAX_MESSAGE_LEN: usize = 65535;

#[repr(u8)]
pub enum MessageType {
    Bytes = 0,
    SnarkOS,
    Kadmium,
}

impl TryFrom<u8> for MessageType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MessageType::Bytes),
            1 => Ok(MessageType::SnarkOS),
            2 => Ok(MessageType::Kadmium),
            _ => Err(format!("u8 value: {value} doesn't correspond to a message variant")),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MessageOrBytes {
    Bytes(Bytes),
    Message(Box<Message<CurrentNetwork>>),
    KadmiumMessage(KadmiumMessage),
}

impl MessageOrBytes {
    fn message_type(&self) -> MessageType {
        match self {
            MessageOrBytes::Bytes(_) => MessageType::Bytes,
            MessageOrBytes::Message(_) => MessageType::SnarkOS,
            MessageOrBytes::KadmiumMessage(_) => MessageType::Kadmium,
        }
    }
}

#[derive(Clone)]
pub struct PostHandshakeState {
    state: Arc<StatelessTransportState>,
    tx_nonce: u64,
    rx_nonce: u64,
}

pub enum NoiseState {
    Handshake(Box<HandshakeState>),
    PostHandshake(PostHandshakeState),
}

impl Clone for NoiseState {
    fn clone(&self) -> Self {
        match self {
            Self::Handshake(..) => unimplemented!(),
            Self::PostHandshake(ph_state) => Self::PostHandshake(ph_state.clone()),
        }
    }
}

impl NoiseState {
    pub fn into_post_handshake_state(self) -> Self {
        if let Self::Handshake(noise_state) = self {
            let noise_state = noise_state.into_stateless_transport_mode().expect("handshake isn't finished");
            Self::PostHandshake(PostHandshakeState { state: Arc::new(noise_state), tx_nonce: 0, rx_nonce: 0 })
        } else {
            panic!()
        }
    }
}

pub struct NoiseCodec {
    codec: LengthDelimitedCodec,
    kadmium_codec: KadmiumCodec,
    snarkos_codec: MessageCodec<CurrentNetwork>,
    noise_state: NoiseState,
}

impl NoiseCodec {
    pub fn new(noise_state: NoiseState) -> Self {
        Self {
            codec: LengthDelimitedCodec::new(),
            kadmium_codec: KadmiumCodec::new(),
            snarkos_codec: MessageCodec::default(),
            noise_state,
        }
    }
}

impl Encoder<MessageOrBytes> for NoiseCodec {
    type Error = io::Error;

    fn encode(&mut self, message_or_bytes: MessageOrBytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let message_type = message_or_bytes.message_type();

        let ciphertext = match self.noise_state {
            NoiseState::Handshake(ref mut noise) => {
                match message_or_bytes {
                    // Don't allow message sending before the noise handshake has completed.
                    MessageOrBytes::Message(_) | MessageOrBytes::KadmiumMessage(_) => unimplemented!(),
                    MessageOrBytes::Bytes(bytes) => {
                        let mut buffer = [0u8; MAX_MESSAGE_LEN + 1];
                        let len = noise
                            .write_message(&bytes, &mut buffer[1..])
                            .map_err(|e| Self::Error::new(io::ErrorKind::InvalidInput, e))?;

                        // Set the message type flag.
                        buffer[0] = message_type as u8;

                        buffer[..len + 1].into()
                    }
                }
            }

            NoiseState::PostHandshake(ref mut noise) => {
                // Encode the message using the appropriate codec.
                let mut bytes = BytesMut::new();
                match message_or_bytes {
                    // Don't allow sending raw bytes after the noise handshake has completed.
                    MessageOrBytes::Bytes(_) => unimplemented!(),
                    MessageOrBytes::Message(message) => self.snarkos_codec.encode(*message, &mut bytes)?,
                    MessageOrBytes::KadmiumMessage(message) => self.kadmium_codec.encode(message, &mut bytes)?,
                }

                // Chunk the payload if necessary.
                let chunked_plaintext_msg: Vec<_> = bytes.chunks(MAX_MESSAGE_LEN - 16).collect();
                let num_chunks = chunked_plaintext_msg.len() as u64;

                // Encrypt the resulting bytes with Noise.
                let encrypted_chunks: Vec<io::Result<Vec<u8>>> = chunked_plaintext_msg
                    .into_par_iter()
                    .enumerate()
                    .map(|(nonce_offset, plaintext_chunk)| {
                        let mut buffer = vec![0u8; MAX_MESSAGE_LEN];

                        let len = noise
                            .state
                            .write_message(noise.tx_nonce + nonce_offset as u64, plaintext_chunk, &mut buffer)
                            .map_err(|e| Self::Error::new(io::ErrorKind::InvalidInput, e))?;

                        buffer.truncate(len);

                        Ok(buffer)
                    })
                    .collect();

                let mut buffer = BytesMut::new();
                // Set the message type flag.
                buffer.put_u8(message_type as u8);

                for chunk in encrypted_chunks {
                    buffer.extend_from_slice(&chunk?)
                }

                noise.tx_nonce += num_chunks;

                buffer
            }
        };

        // Encode the resulting ciphertext using the length-delimited codec.
        self.codec.encode(ciphertext.freeze(), dst)
    }
}

impl Decoder for NoiseCodec {
    type Error = io::Error;
    type Item = MessageOrBytes;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Decode the ciphertext with the length-delimited codec.
        let (flag, bytes) = if let Some(mut bytes) = self.codec.decode(src)? {
            let flag =
                MessageType::try_from(bytes.get_u8()).map_err(|e| Self::Error::new(io::ErrorKind::InvalidData, e))?;
            (flag, bytes)
        } else {
            return Ok(None);
        };

        let msg = match self.noise_state {
            NoiseState::Handshake(ref mut noise) => {
                if let MessageType::SnarkOS | MessageType::Kadmium = flag {
                    // Ignore any messages before the noise handshake has completed.
                    return Ok(None);
                }

                // Decrypt the ciphertext in handshake mode.
                let mut buffer = [0u8; MAX_MESSAGE_LEN];
                let len = noise.read_message(&bytes, &mut buffer).map_err(|_| io::ErrorKind::InvalidData)?;

                Some(MessageOrBytes::Bytes(Bytes::copy_from_slice(&buffer[..len])))
            }

            NoiseState::PostHandshake(ref mut noise) => {
                // Ignore raw bytes after the noise handshake has completed.
                if let MessageType::Bytes = flag {
                    return Ok(None);
                }

                // Noise decryption.
                let chunked_encrypted_msg: Vec<_> = bytes.chunks(MAX_MESSAGE_LEN).collect();
                let num_chunks = chunked_encrypted_msg.len() as u64;

                let decrypted_chunks: Vec<io::Result<Vec<u8>>> = chunked_encrypted_msg
                    .into_par_iter()
                    .enumerate()
                    .map(|(nonce_offset, encrypted_chunk)| {
                        let mut buffer = vec![0u8; MAX_MESSAGE_LEN];

                        // Decrypt the ciphertext in post-handshake mode.
                        let len = noise
                            .state
                            .read_message(noise.rx_nonce + nonce_offset as u64, encrypted_chunk, &mut buffer)
                            .map_err(|_| io::ErrorKind::InvalidData)?;

                        buffer.truncate(len);
                        Ok(buffer)
                    })
                    .collect();

                noise.rx_nonce += num_chunks;

                // Collect chunks into plaintext to be passed to the message codecs.
                let mut plaintext = BytesMut::new();
                for chunk in decrypted_chunks {
                    plaintext.extend_from_slice(&chunk?);
                }

                // Decode with message codecs.
                match flag {
                    MessageType::SnarkOS => {
                        self.snarkos_codec.decode(&mut plaintext)?.map(|msg| MessageOrBytes::Message(Box::new(msg)))
                    }
                    MessageType::Kadmium => {
                        self.kadmium_codec.decode(&mut plaintext)?.map(MessageOrBytes::KadmiumMessage)
                    }
                    _ => unreachable!("bytes variant was handled as an early return"),
                }
            }
        };

        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm::prelude::{Address, Group, TestRng, Uniform};

    use crate::{
        BlockRequest,
        ChallengeRequest,
        Disconnect,
        DisconnectReason,
        NodeType,
        PeerRequest,
        PeerResponse,
        Ping,
        Pong,
        PuzzleRequest,
    };
    use snow::{params::NoiseParams, Builder};

    fn handshake_xx() -> (NoiseCodec, NoiseCodec) {
        let params: NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2s".parse().unwrap();

        let initiator_builder = Builder::new(params.clone());
        let initiator_kp = initiator_builder.generate_keypair().unwrap();
        let initiator = initiator_builder.local_private_key(&initiator_kp.private).build_initiator().unwrap();

        let responder_builder = Builder::new(params);
        let responder_kp = responder_builder.generate_keypair().unwrap();
        let responder = responder_builder.local_private_key(&responder_kp.private).build_responder().unwrap();

        let mut initiator_codec = NoiseCodec::new(NoiseState::Handshake(Box::new(initiator)));
        let mut responder_codec = NoiseCodec::new(NoiseState::Handshake(Box::new(responder)));

        let mut ciphertext = BytesMut::new();

        // -> e
        assert!(initiator_codec.encode(MessageOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
        assert!(
            matches!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), MessageOrBytes::Bytes(bytes) if bytes.is_empty())
        );

        // <- e, ee, s, es
        assert!(responder_codec.encode(MessageOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
        assert!(
            matches!(initiator_codec.decode(&mut ciphertext).unwrap().unwrap(), MessageOrBytes::Bytes(bytes) if bytes.is_empty())
        );

        // -> s, se
        assert!(initiator_codec.encode(MessageOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
        assert!(
            matches!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), MessageOrBytes::Bytes(bytes) if bytes.is_empty())
        );

        initiator_codec.noise_state = initiator_codec.noise_state.into_post_handshake_state();
        responder_codec.noise_state = responder_codec.noise_state.into_post_handshake_state();

        (initiator_codec, responder_codec)
    }

    fn assert_roundtrip(msg: MessageOrBytes) {
        let (mut initiator_codec, mut responder_codec) = handshake_xx();
        let mut ciphertext = BytesMut::new();

        assert!(initiator_codec.encode(msg.clone(), &mut ciphertext).is_ok());
        assert_eq!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), msg);
    }

    #[test]
    fn block_request_roundtrip() {
        let block_request =
            MessageOrBytes::Message(Box::new(Message::BlockRequest(BlockRequest { start_height: 0, end_height: 100 })));

        assert_roundtrip(block_request);
    }

    #[test]
    fn challenge_request_roundtrip() {
        let rng = &mut TestRng::default();

        let challenge_request = MessageOrBytes::Message(Box::new(Message::ChallengeRequest(ChallengeRequest {
            version: 0,
            listener_port: 0,
            node_type: NodeType::Client,
            address: Address::new(Group::rand(rng)),
            nonce: 0,
        })));

        assert_roundtrip(challenge_request);
    }

    #[test]
    fn disconnect_roundtrip() {
        let disconnect = MessageOrBytes::Message(Box::new(Message::Disconnect(Disconnect {
            reason: DisconnectReason::NoReasonGiven,
        })));

        assert_roundtrip(disconnect);
    }

    #[test]
    fn peer_request_roundtrip() {
        let peer_request = MessageOrBytes::Message(Box::new(Message::PeerRequest(PeerRequest)));
        assert_roundtrip(peer_request);
    }

    #[test]
    fn peer_response_roundtrip() {
        let peer_response = MessageOrBytes::Message(Box::new(Message::PeerResponse(PeerResponse { peers: vec![] })));
        assert_roundtrip(peer_response);
    }

    #[test]
    fn ping_roundtrip() {
        let ping = MessageOrBytes::Message(Box::new(Message::Ping(Ping::<CurrentNetwork> {
            version: 0,
            node_type: NodeType::Client,
            block_locators: None,
        })));

        assert_roundtrip(ping)
    }

    #[test]
    fn pong_roundtrip() {
        let pong = MessageOrBytes::Message(Box::new(Message::Pong(Pong { is_fork: Some(true) })));
        assert_roundtrip(pong);
    }

    #[test]
    fn puzzle_request_roundtrip() {
        let puzzle_request = MessageOrBytes::Message(Box::new(Message::PuzzleRequest(PuzzleRequest)));
        assert_roundtrip(puzzle_request);
    }
}
