// Copyright 2024 Aleo Network Foundation
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

use bytes::{Buf, BufMut, BytesMut};
use core::marker::PhantomData;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};
use tracing::*;

/// The maximum size of an event that can be transmitted during the handshake.
const MAX_HANDSHAKE_SIZE: usize = 1024 * 1024; // 1 MiB
/// The maximum size of an event that can be transmitted in the network.
const MAX_EVENT_SIZE: usize = 256 * 1024 * 1024; // 256 MiB

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prop_tests::any_event;
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    fn assert_roundtrip(msg: Event<CurrentNetwork>) {
        let mut codec: EventCodec<CurrentNetwork> = Default::default();
        let mut encoded_event = BytesMut::new();

        assert!(codec.encode(msg.clone(), &mut encoded_event).is_ok());
        let decoded = codec.decode(&mut encoded_event).unwrap().unwrap();
        assert_eq!(decoded.to_bytes_le().unwrap(), msg.to_bytes_le().unwrap());
    }

    #[proptest]
    fn event_roundtrip(#[strategy(any_event())] event: Event<CurrentNetwork>) {
        assert_roundtrip(event)
    }
}
