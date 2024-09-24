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

use crate::Message;
use snarkvm::prelude::{FromBytes, Network, ToBytes};

use ::bytes::{Buf, BufMut, BytesMut};
use core::marker::PhantomData;
use tokio_util::codec::{Decoder, Encoder, LengthDelimitedCodec};

/// The maximum size of a message that can be transmitted during the handshake.
const MAXIMUM_HANDSHAKE_MESSAGE_SIZE: usize = 1024 * 1024; // 1 MiB

/// The maximum size of a message that can be transmitted in the network.
pub(crate) const MAXIMUM_MESSAGE_SIZE: usize = 128 * 1024 * 1024; // 128 MiB

/// The codec used to decode and encode network `Message`s.
pub struct MessageCodec<N: Network> {
    codec: LengthDelimitedCodec,
    _phantom: PhantomData<N>,
}

impl<N: Network> MessageCodec<N> {
    pub fn handshake() -> Self {
        let mut codec = Self::default();
        codec.codec.set_max_frame_length(MAXIMUM_HANDSHAKE_MESSAGE_SIZE);
        codec
    }
}

impl<N: Network> Default for MessageCodec<N> {
    fn default() -> Self {
        Self {
            codec: LengthDelimitedCodec::builder().max_frame_length(MAXIMUM_MESSAGE_SIZE).little_endian().new_codec(),
            _phantom: Default::default(),
        }
    }
}

impl<N: Network> Encoder<Message<N>> for MessageCodec<N> {
    type Error = std::io::Error;

    fn encode(&mut self, message: Message<N>, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // Serialize the payload directly into dst.
        message
            .write_le(&mut dst.writer())
            // This error should never happen, the conversion is for greater compatibility.
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "serialization error"))?;

        let serialized_message = dst.split_to(dst.len()).freeze();

        self.codec.encode(serialized_message, dst)
    }
}

impl<N: Network> Decoder for MessageCodec<N> {
    type Error = std::io::Error;
    type Item = Message<N>;

    fn decode(&mut self, source: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Decode a frame containing bytes belonging to a message.
        let bytes = match self.codec.decode(source)? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };

        Self::Item::check_size(&bytes)?;

        // Convert the bytes to a message, or fail if it is not valid.
        let reader = bytes.reader();
        match Message::read_le(reader) {
            Ok(message) => Ok(Some(message)),
            Err(error) => {
                warn!("Failed to deserialize a message - {}", error);
                Err(std::io::ErrorKind::InvalidData.into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        unconfirmed_transaction::prop_tests::{any_large_unconfirmed_transaction, any_unconfirmed_transaction},
        UnconfirmedTransaction,
    };

    use proptest::prelude::ProptestConfig;
    use test_strategy::proptest;

    type CurrentNetwork = snarkvm::prelude::MainnetV0;

    #[proptest]
    fn unconfirmed_transaction(#[strategy(any_unconfirmed_transaction())] tx: UnconfirmedTransaction<CurrentNetwork>) {
        let mut bytes = BytesMut::new();
        let mut codec = MessageCodec::<CurrentNetwork>::default();
        assert!(codec.encode(Message::UnconfirmedTransaction(tx), &mut bytes).is_ok());
        assert!(codec.decode(&mut bytes).is_ok());
    }

    #[proptest(ProptestConfig { cases : 10, ..ProptestConfig::default() })]
    fn overly_large_unconfirmed_transaction(
        #[strategy(any_large_unconfirmed_transaction())] tx: UnconfirmedTransaction<CurrentNetwork>,
    ) {
        let mut bytes = BytesMut::new();
        let mut codec = MessageCodec::<CurrentNetwork>::default();
        assert!(codec.encode(Message::UnconfirmedTransaction(tx), &mut bytes).is_ok());
        assert!(matches!(codec.decode(&mut bytes), Err(err) if err.kind() == std::io::ErrorKind::InvalidData));
    }
}
