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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

pub mod helpers;
pub use helpers::*;

mod block_request;
pub use block_request::BlockRequest;

mod block_response;
pub use block_response::BlockResponse;

mod challenge_request;
pub use challenge_request::ChallengeRequest;

mod challenge_response;
pub use challenge_response::ChallengeResponse;

mod disconnect;
pub use disconnect::Disconnect;

mod peer_request;
pub use peer_request::PeerRequest;

mod peer_response;
pub use peer_response::PeerResponse;

mod ping;
pub use ping::Ping;

mod pong;
pub use pong::Pong;

mod puzzle_request;
pub use puzzle_request::PuzzleRequest;

mod puzzle_response;
pub use puzzle_response::PuzzleResponse;

mod unconfirmed_solution;
pub use unconfirmed_solution::UnconfirmedSolution;

mod unconfirmed_transaction;
pub use unconfirmed_transaction::UnconfirmedTransaction;

pub use snarkos_node_bft_events::DataBlocks;

use snarkos_node_sync_locators::BlockLocators;
use snarkvm::prelude::{
    block::{Header, Transaction},
    error,
    puzzle::{Solution, SolutionID},
    Address,
    FromBytes,
    Network,
    Signature,
    ToBytes,
};

use std::{
    borrow::Cow,
    fmt,
    fmt::{Display, Formatter},
    io,
    net::SocketAddr,
};

pub trait MessageTrait: ToBytes + FromBytes {
    /// Returns the message name.
    fn name(&self) -> Cow<'static, str>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message<N: Network> {
    BlockRequest(BlockRequest),
    BlockResponse(BlockResponse<N>),
    ChallengeRequest(ChallengeRequest<N>),
    ChallengeResponse(ChallengeResponse<N>),
    Disconnect(Disconnect),
    PeerRequest(PeerRequest),
    PeerResponse(PeerResponse),
    Ping(Ping<N>),
    Pong(Pong),
    PuzzleRequest(PuzzleRequest),
    PuzzleResponse(PuzzleResponse<N>),
    UnconfirmedSolution(UnconfirmedSolution<N>),
    UnconfirmedTransaction(UnconfirmedTransaction<N>),
}

impl<N: Network> From<DisconnectReason> for Message<N> {
    fn from(reason: DisconnectReason) -> Self {
        Self::Disconnect(Disconnect { reason })
    }
}

impl<N: Network> Message<N> {
    /// The version of the network protocol; it can be incremented in order to force users to update.
    pub const VERSION: u32 = 17;

    /// Returns the message name.
    #[inline]
    pub fn name(&self) -> Cow<'static, str> {
        match self {
            Self::BlockRequest(message) => message.name(),
            Self::BlockResponse(message) => message.name(),
            Self::ChallengeRequest(message) => message.name(),
            Self::ChallengeResponse(message) => message.name(),
            Self::Disconnect(message) => message.name(),
            Self::PeerRequest(message) => message.name(),
            Self::PeerResponse(message) => message.name(),
            Self::Ping(message) => message.name(),
            Self::Pong(message) => message.name(),
            Self::PuzzleRequest(message) => message.name(),
            Self::PuzzleResponse(message) => message.name(),
            Self::UnconfirmedSolution(message) => message.name(),
            Self::UnconfirmedTransaction(message) => message.name(),
        }
    }

    /// Returns the message ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::BlockRequest(..) => 0,
            Self::BlockResponse(..) => 1,
            Self::ChallengeRequest(..) => 2,
            Self::ChallengeResponse(..) => 3,
            Self::Disconnect(..) => 4,
            Self::PeerRequest(..) => 5,
            Self::PeerResponse(..) => 6,
            Self::Ping(..) => 7,
            Self::Pong(..) => 8,
            Self::PuzzleRequest(..) => 9,
            Self::PuzzleResponse(..) => 10,
            Self::UnconfirmedSolution(..) => 11,
            Self::UnconfirmedTransaction(..) => 12,
        }
    }

    /// Checks the message byte length. To be used before deserialization.
    pub fn check_size(bytes: &[u8]) -> io::Result<()> {
        // Store the length to be checked against the max message size for each variant.
        let len = bytes.len();
        if len < 2 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "invalid message"));
        }

        // Check the first two bytes for the message ID.
        let id_bytes: [u8; 2] = (&bytes[..2])
            .try_into()
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "id couldn't be deserialized"))?;
        let id = u16::from_le_bytes(id_bytes);

        // SPECIAL CASE: check the transaction message isn't too large.
        if id == 12 && len > N::MAX_TRANSACTION_SIZE {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "transaction is too large"))?;
        }

        Ok(())
    }
}

impl<N: Network> ToBytes for Message<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.id().write_le(&mut writer)?;

        match self {
            Self::BlockRequest(message) => message.write_le(writer),
            Self::BlockResponse(message) => message.write_le(writer),
            Self::ChallengeRequest(message) => message.write_le(writer),
            Self::ChallengeResponse(message) => message.write_le(writer),
            Self::Disconnect(message) => message.write_le(writer),
            Self::PeerRequest(message) => message.write_le(writer),
            Self::PeerResponse(message) => message.write_le(writer),
            Self::Ping(message) => message.write_le(writer),
            Self::Pong(message) => message.write_le(writer),
            Self::PuzzleRequest(message) => message.write_le(writer),
            Self::PuzzleResponse(message) => message.write_le(writer),
            Self::UnconfirmedSolution(message) => message.write_le(writer),
            Self::UnconfirmedTransaction(message) => message.write_le(writer),
        }
    }
}

impl<N: Network> FromBytes for Message<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self> {
        // Read the event ID.
        let mut id_bytes = [0u8; 2];
        reader.read_exact(&mut id_bytes)?;
        let id = u16::from_le_bytes(id_bytes);

        // Deserialize the data field.
        let message = match id {
            0 => Self::BlockRequest(BlockRequest::read_le(&mut reader)?),
            1 => Self::BlockResponse(BlockResponse::read_le(&mut reader)?),
            2 => Self::ChallengeRequest(ChallengeRequest::read_le(&mut reader)?),
            3 => Self::ChallengeResponse(ChallengeResponse::read_le(&mut reader)?),
            4 => Self::Disconnect(Disconnect::read_le(&mut reader)?),
            5 => Self::PeerRequest(PeerRequest::read_le(&mut reader)?),
            6 => Self::PeerResponse(PeerResponse::read_le(&mut reader)?),
            7 => Self::Ping(Ping::read_le(&mut reader)?),
            8 => Self::Pong(Pong::read_le(&mut reader)?),
            9 => Self::PuzzleRequest(PuzzleRequest::read_le(&mut reader)?),
            10 => Self::PuzzleResponse(PuzzleResponse::read_le(&mut reader)?),
            11 => Self::UnconfirmedSolution(UnconfirmedSolution::read_le(&mut reader)?),
            12 => Self::UnconfirmedTransaction(UnconfirmedTransaction::read_le(&mut reader)?),
            13.. => return Err(error("Unknown message ID {id}")),
        };

        // Ensure that there are no "dangling" bytes.
        if reader.bytes().next().is_some() {
            return Err(error("Leftover bytes in a Message"));
        }

        Ok(message)
    }
}
