// Copyright (C) 2019-2023 Aleo Systems Inc.
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::Event;
use snarkvm::prelude::{FromBytes, Network, ToBytes};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use core::marker::PhantomData;
use rayon::{
    iter::{IndexedParallelIterator, ParallelIterator},
    prelude::ParallelSlice,
};
use snow::{HandshakeState, StatelessTransportState};
use std::{io, sync::Arc};
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
use tracing::*;

/// The maximum size of an event that can be transmitted during the handshake.
const MAX_HANDSHAKE_SIZE: usize = 1024 * 1024; // 1 MiB
/// The maximum size of an event that can be transmitted in the network.
const MAX_EVENT_SIZE: usize = 128 * 1024 * 1024; // 128 MiB

/// The type of noise handshake to use for network encryption.
pub const NOISE_HANDSHAKE_TYPE: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";

/// The codec used to decode and encode network `Event`s.
pub struct EventCodec<N: Network> {
    codec: LengthDelimitedCodec,
    _phantom: PhantomData<N>,
}

impl<N: Network> EventCodec<N> {
    pub fn handshake() -> Self {
        let mut codec = Self::default();
        codec.codec.set_max_frame_length(MAX_HANDSHAKE_SIZE);
        codec
    }
}

impl<N: Network> Default for EventCodec<N> {
    fn default() -> Self {
        Self {
            codec: LengthDelimitedCodec::builder().max_frame_length(MAX_EVENT_SIZE).little_endian().new_codec(),
            _phantom: Default::default(),
        }
    }
}

impl<N: Network> Encoder<Event<N>> for EventCodec<N> {
    type Error = std::io::Error;

    fn encode(&mut self, event: Event<N>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize the payload directly into dst.
        event
            .write_le(&mut dst.writer())
            // This error should never happen, the conversion is for greater compatibility.
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "serialization error"))?;

        let serialized_event = dst.split_to(dst.len()).freeze();

        self.codec.encode(serialized_event, dst)
    }
}

impl<N: Network> Decoder for EventCodec<N> {
    type Error = std::io::Error;
    type Item = Event<N>;

    fn decode(&mut self, source: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Decode a frame containing bytes belonging to an event.
        let bytes = match self.codec.decode(source)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        // Convert the bytes to an event, or fail if it is not valid.
        let reader = bytes.reader();
        match Event::read_le(reader) {
            Ok(event) => Ok(Some(event)),
            Err(error) => {
                error!("Failed to deserialize an event: {}", error);
                Err(std::io::ErrorKind::InvalidData.into())
            }
        }
    }
}

/* NOISE CODEC */

// The maximum message size for noise messages. If the data to be encrypted exceeds it, it is chunked.
const MAX_MESSAGE_LEN: usize = 65535;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventOrBytes<N: Network> {
    Bytes(Bytes),
    Event(Event<N>),
}

impl<N: Network> ToBytes for EventOrBytes<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        match self {
            Self::Bytes(bytes) => {
                0u8.write_le(&mut writer)?;
                writer.write_all(bytes)
            }
            Self::Event(event) => {
                1u8.write_le(&mut writer)?;
                event.write_le(writer)
            }
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
    Failed,
}

impl Clone for NoiseState {
    fn clone(&self) -> Self {
        match self {
            Self::Handshake(..) => unreachable!(),
            Self::PostHandshake(ph_state) => Self::PostHandshake(ph_state.clone()),
            Self::Failed => unreachable!("Forbidden: cloning noise handshake"),
        }
    }
}

impl NoiseState {
    pub fn into_post_handshake_state(self) -> Self {
        if let Self::Handshake(noise_state) = self {
            match noise_state.into_stateless_transport_mode() {
                Ok(new_state) => {
                    return Self::PostHandshake(PostHandshakeState {
                        state: Arc::new(new_state),
                        tx_nonce: 0,
                        rx_nonce: 0,
                    });
                }
                Err(error) => {
                    warn!("Handshake not finished - {error}");
                }
            }
        } else {
            warn!("Handshake in wrong state");
        }

        NoiseState::Failed
    }
}

pub struct NoiseCodec<N: Network> {
    codec: LengthDelimitedCodec,
    event_codec: EventCodec<N>,
    pub noise_state: NoiseState,
}

impl<N: Network> NoiseCodec<N> {
    pub fn new(noise_state: NoiseState) -> Self {
        Self { codec: LengthDelimitedCodec::new(), event_codec: EventCodec::default(), noise_state }
    }
}

impl<N: Network> Encoder<EventOrBytes<N>> for NoiseCodec<N> {
    type Error = std::io::Error;

    fn encode(&mut self, message_or_bytes: EventOrBytes<N>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        #[cfg(feature = "metrics")]
        let start = std::time::Instant::now();

        let ciphertext = match self.noise_state {
            NoiseState::Handshake(ref mut noise) => {
                match message_or_bytes {
                    // Don't allow message sending before the noise handshake has completed.
                    EventOrBytes::Event(_) => unimplemented!(),
                    EventOrBytes::Bytes(bytes) => {
                        let mut buffer = [0u8; MAX_MESSAGE_LEN];
                        let len = noise
                            .write_message(&bytes, &mut buffer[..])
                            .map_err(|e| Self::Error::new(io::ErrorKind::InvalidInput, e))?;

                        #[cfg(feature = "metrics")]
                        metrics::histogram(metrics::tcp::NOISE_CODEC_ENCRYPTION_SIZE, len as f64);

                        buffer[..len].into()
                    }
                }
            }

            NoiseState::PostHandshake(ref mut noise) => {
                // Encode the message using the event codec.
                let mut bytes = BytesMut::new();
                match message_or_bytes {
                    // Don't allow sending raw bytes after the noise handshake has completed.
                    EventOrBytes::Bytes(_) => panic!("Unsupported post-handshake"),
                    EventOrBytes::Event(event) => self.event_codec.encode(event, &mut bytes)?,
                }

                #[cfg(feature = "metrics")]
                metrics::histogram(metrics::tcp::NOISE_CODEC_ENCRYPTION_SIZE, bytes.len() as f64);

                // Chunk the payload if necessary and encrypt with Noise.
                //
                // A Noise transport message is simply an AEAD ciphertext that is less than or
                // equal to 65535 bytes in length, and that consists of an encrypted payload plus
                // 16 bytes of authentication data.
                //
                // See: https://noiseprotocol.org/noise.html#the-handshakestate-object
                const TAG_LEN: usize = 16;
                let encrypted_chunks = bytes
                    .par_chunks(MAX_MESSAGE_LEN - TAG_LEN)
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
                    .collect::<io::Result<Vec<Vec<u8>>>>()?;

                let mut buffer = BytesMut::with_capacity(encrypted_chunks.len());
                for chunk in encrypted_chunks {
                    buffer.extend_from_slice(&chunk);
                    noise.tx_nonce += 1;
                }

                buffer
            }

            NoiseState::Failed => unreachable!("Noise handshake failed to encode"),
        };

        // Encode the resulting ciphertext using the length-delimited codec.
        #[allow(clippy::let_and_return)]
        let result = self.codec.encode(ciphertext.freeze(), dst);

        #[cfg(feature = "metrics")]
        metrics::histogram(metrics::tcp::NOISE_CODEC_ENCRYPTION_TIME, start.elapsed().as_micros() as f64);
        result
    }
}

impl<N: Network> Decoder for NoiseCodec<N> {
    type Error = io::Error;
    type Item = EventOrBytes<N>;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        #[cfg(feature = "metrics")]
        metrics::histogram(metrics::tcp::NOISE_CODEC_DECRYPTION_SIZE, src.len() as f64);
        #[cfg(feature = "metrics")]
        let start = std::time::Instant::now();

        // Decode the ciphertext with the length-delimited codec.
        let Some(bytes) = self.codec.decode(src)? else {
            return Ok(None);
        };

        let msg = match self.noise_state {
            NoiseState::Handshake(ref mut noise) => {
                // Decrypt the ciphertext in handshake mode.
                let mut buffer = [0u8; MAX_MESSAGE_LEN];
                let len = noise.read_message(&bytes, &mut buffer).map_err(|_| io::ErrorKind::InvalidData)?;

                Some(EventOrBytes::Bytes(Bytes::copy_from_slice(&buffer[..len])))
            }

            NoiseState::PostHandshake(ref mut noise) => {
                // Noise decryption.
                let decrypted_chunks = bytes
                    .par_chunks(MAX_MESSAGE_LEN)
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
                    .collect::<io::Result<Vec<Vec<u8>>>>()?;

                // Collect chunks into plaintext to be passed to the message codecs.
                let mut plaintext = BytesMut::new();
                for chunk in decrypted_chunks {
                    plaintext.extend_from_slice(&chunk);
                    noise.rx_nonce += 1;
                }

                // Decode with message codecs.
                self.event_codec.decode(&mut plaintext)?.map(|msg| EventOrBytes::Event(msg))
            }

            NoiseState::Failed => unreachable!("Noise handshake failed to decode"),
        };

        #[cfg(feature = "metrics")]
        metrics::histogram(metrics::tcp::NOISE_CODEC_DECRYPTION_TIME, start.elapsed().as_micros() as f64);
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prop_tests::any_event;

    use snow::{params::NoiseParams, Builder};
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::Testnet3;

    fn handshake_xx() -> (NoiseCodec<CurrentNetwork>, NoiseCodec<CurrentNetwork>) {
        let params: NoiseParams = NOISE_HANDSHAKE_TYPE.parse().unwrap();
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
        assert!(initiator_codec.encode(EventOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
        assert!(
            matches!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), EventOrBytes::Bytes(bytes) if bytes.is_empty())
        );

        // <- e, ee, s, es
        assert!(responder_codec.encode(EventOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
        assert!(
            matches!(initiator_codec.decode(&mut ciphertext).unwrap().unwrap(), EventOrBytes::Bytes(bytes) if bytes.is_empty())
        );

        // -> s, se
        assert!(initiator_codec.encode(EventOrBytes::Bytes(Bytes::new()), &mut ciphertext).is_ok());
        assert!(
            matches!(responder_codec.decode(&mut ciphertext).unwrap().unwrap(), EventOrBytes::Bytes(bytes) if bytes.is_empty())
        );

        initiator_codec.noise_state = initiator_codec.noise_state.into_post_handshake_state();
        responder_codec.noise_state = responder_codec.noise_state.into_post_handshake_state();

        (initiator_codec, responder_codec)
    }

    fn assert_roundtrip(msg: EventOrBytes<CurrentNetwork>) {
        let (mut initiator_codec, mut responder_codec) = handshake_xx();
        let mut ciphertext = BytesMut::new();

        assert!(initiator_codec.encode(msg.clone(), &mut ciphertext).is_ok());
        let decoded = responder_codec.decode(&mut ciphertext).unwrap().unwrap();
        assert_eq!(decoded.to_bytes_le().unwrap(), msg.to_bytes_le().unwrap());
    }

    #[proptest]
    fn event_roundtrip(#[strategy(any_event())] event: Event<CurrentNetwork>) {
        assert_roundtrip(EventOrBytes::Event(event))
    }
}
