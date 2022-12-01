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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

pub mod helpers;
pub use helpers::*;

mod beacon_propose;
pub use beacon_propose::BeaconPropose;

mod beacon_timeout;
pub use beacon_timeout::BeaconTimeout;

mod beacon_vote;
pub use beacon_vote::BeaconVote;

mod block_request;
pub use block_request::BlockRequest;

mod block_response;
pub use block_response::{BlockResponse, DataBlocks};

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

use snarkvm::prelude::{
    error,
    Address,
    Block,
    EpochChallenge,
    FromBytes,
    Header,
    Network,
    ProverSolution,
    PuzzleCommitment,
    Signature,
    ToBytes,
    Transaction,
};

use ::bytes::{Buf, BytesMut};
use anyhow::{bail, Result};
use std::{
    fmt,
    fmt::{Display, Formatter},
    io::{Read, Result as IoResult, Write},
    net::SocketAddr,
    ops::Deref,
};

pub trait MessageTrait {
    /// Returns the message name.
    fn name(&self) -> String;
    /// Serializes the message into the buffer.
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()>;
    /// Deserializes the given buffer into a message.
    fn deserialize(bytes: BytesMut) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Message<N: Network> {
    BeaconPropose(BeaconPropose<N>),
    BeaconTimeout(BeaconTimeout<N>),
    BeaconVote(BeaconVote<N>),
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

impl<N: Network> Message<N> {
    /// The version of the network protocol; it can be incremented in order to force users to update.
    pub const VERSION: u32 = 4;

    /// Returns the message name.
    #[inline]
    pub fn name(&self) -> String {
        match self {
            Self::BeaconPropose(message) => message.name(),
            Self::BeaconTimeout(message) => message.name(),
            Self::BeaconVote(message) => message.name(),
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
            Self::BeaconPropose(..) => 0,
            Self::BeaconTimeout(..) => 1,
            Self::BeaconVote(..) => 2,
            Self::BlockRequest(..) => 3,
            Self::BlockResponse(..) => 4,
            Self::ChallengeRequest(..) => 5,
            Self::ChallengeResponse(..) => 6,
            Self::Disconnect(..) => 7,
            Self::PeerRequest(..) => 8,
            Self::PeerResponse(..) => 9,
            Self::Ping(..) => 10,
            Self::Pong(..) => 11,
            Self::PuzzleRequest(..) => 12,
            Self::PuzzleResponse(..) => 13,
            Self::UnconfirmedSolution(..) => 14,
            Self::UnconfirmedTransaction(..) => 15,
        }
    }

    /// Serializes the message into the buffer.
    #[inline]
    pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.id().to_le_bytes()[..])?;

        match self {
            Self::BeaconPropose(message) => message.serialize(writer),
            Self::BeaconTimeout(message) => message.serialize(writer),
            Self::BeaconVote(message) => message.serialize(writer),
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
            0 => Self::BeaconPropose(MessageTrait::deserialize(bytes)?),
            1 => Self::BeaconTimeout(MessageTrait::deserialize(bytes)?),
            2 => Self::BeaconVote(MessageTrait::deserialize(bytes)?),
            3 => Self::BlockRequest(MessageTrait::deserialize(bytes)?),
            4 => Self::BlockResponse(MessageTrait::deserialize(bytes)?),
            5 => Self::ChallengeRequest(MessageTrait::deserialize(bytes)?),
            6 => Self::ChallengeResponse(MessageTrait::deserialize(bytes)?),
            7 => Self::Disconnect(MessageTrait::deserialize(bytes)?),
            8 => Self::PeerRequest(MessageTrait::deserialize(bytes)?),
            9 => Self::PeerResponse(MessageTrait::deserialize(bytes)?),
            10 => Self::Ping(MessageTrait::deserialize(bytes)?),
            11 => Self::Pong(MessageTrait::deserialize(bytes)?),
            12 => Self::PuzzleRequest(MessageTrait::deserialize(bytes)?),
            13 => Self::PuzzleResponse(MessageTrait::deserialize(bytes)?),
            14 => Self::UnconfirmedSolution(MessageTrait::deserialize(bytes)?),
            15 => Self::UnconfirmedTransaction(MessageTrait::deserialize(bytes)?),
            _ => bail!("Unknown message ID {id}"),
        };

        Ok(message)
    }
}
