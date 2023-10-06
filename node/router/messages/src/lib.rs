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

pub use snarkos_node_narwhal_events::DataBlocks;

use snarkos_node_sync_locators::BlockLocators;
use snarkvm::prelude::{
    block::{Header, Transaction},
    coinbase::{EpochChallenge, ProverSolution, PuzzleCommitment},
    Address,
    FromBytes,
    Network,
    Signature,
    ToBytes,
};

use ::bytes::{Buf, BytesMut};
use anyhow::{bail, Result};
use std::{
    borrow::Cow,
    fmt,
    fmt::{Display, Formatter},
    io::Write,
    net::SocketAddr,
};

pub trait MessageTrait {
    /// Returns the message name.
    fn name(&self) -> Cow<'static, str>;
    /// Serializes the message into the buffer.
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()>;
    /// Deserializes the given buffer into a message.
    fn deserialize(bytes: BytesMut) -> Result<Self>
    where
        Self: Sized;
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
    pub const VERSION: u32 = 10;

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

    /// Serializes the message into the buffer.
    #[inline]
    pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.id().write_le(&mut *writer)?;

        match self {
            Self::BlockRequest(message) => message.serialize(writer),
            Self::BlockResponse(message) => message.serialize(writer),
            Self::ChallengeRequest(message) => message.serialize(writer),
            Self::ChallengeResponse(message) => message.serialize(writer),
            Self::Disconnect(message) => message.serialize(writer),
            Self::PeerRequest(message) => message.serialize(writer),
            Self::PeerResponse(message) => message.serialize(writer),
            Self::Ping(message) => message.serialize(writer),
            Self::Pong(message) => message.serialize(writer),
            Self::PuzzleRequest(message) => message.serialize(writer),
            Self::PuzzleResponse(message) => message.serialize(writer),
            Self::UnconfirmedSolution(message) => message.serialize(writer),
            Self::UnconfirmedTransaction(message) => message.serialize(writer),
        }
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    pub fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        // Ensure there is at least a message ID in the buffer.
        if bytes.remaining() < 2 {
            bail!("Missing message ID");
        }

        // Read the message ID.
        let id: u16 = bytes.get_u16_le();

        // Deserialize the data field.
        let message = match id {
            0 => Self::BlockRequest(MessageTrait::deserialize(bytes)?),
            1 => Self::BlockResponse(MessageTrait::deserialize(bytes)?),
            2 => Self::ChallengeRequest(MessageTrait::deserialize(bytes)?),
            3 => Self::ChallengeResponse(MessageTrait::deserialize(bytes)?),
            4 => Self::Disconnect(MessageTrait::deserialize(bytes)?),
            5 => Self::PeerRequest(MessageTrait::deserialize(bytes)?),
            6 => Self::PeerResponse(MessageTrait::deserialize(bytes)?),
            7 => Self::Ping(MessageTrait::deserialize(bytes)?),
            8 => Self::Pong(MessageTrait::deserialize(bytes)?),
            9 => Self::PuzzleRequest(MessageTrait::deserialize(bytes)?),
            10 => Self::PuzzleResponse(MessageTrait::deserialize(bytes)?),
            11 => Self::UnconfirmedSolution(MessageTrait::deserialize(bytes)?),
            12 => Self::UnconfirmedTransaction(MessageTrait::deserialize(bytes)?),
            13.. => bail!("Unknown message ID {id}"),
        };

        Ok(message)
    }
}
