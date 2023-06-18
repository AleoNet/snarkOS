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

mod challenge_request;
pub use challenge_request::ChallengeRequest;

mod challenge_response;
pub use challenge_response::ChallengeResponse;

mod disconnect;
pub use disconnect::Disconnect;

mod ping;
pub use ping::Ping;

mod worker_batch;
pub use worker_batch::WorkerBatch;

use crate::helpers::EntryID;
use snarkos_node_messages::{Data, DisconnectReason};
use snarkvm::{
    console::prelude::{FromBytes, Network, ToBytes},
    prelude::{Address, Signature},
};

use ::bytes::{Buf, BytesMut};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fmt::{Display, Formatter},
    io::{Read, Result as IoResult, Write},
    net::SocketAddr,
};

pub trait EventTrait {
    /// Returns the event name.
    fn name(&self) -> String;
    /// Serializes the event into the buffer.
    fn serialize<W: Write>(&self, writer: &mut W) -> Result<()>;
    /// Deserializes the given buffer into a event.
    fn deserialize(bytes: BytesMut) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event<N: Network> {
    ChallengeRequest(ChallengeRequest<N>),
    ChallengeResponse(ChallengeResponse<N>),
    Disconnect(Disconnect),
    Ping(Ping<N>),
    WorkerBatch(WorkerBatch<N>),
}

impl<N: Network> Event<N> {
    /// The version of the event protocol; it can be incremented in order to force users to update.
    pub const VERSION: u32 = 1;

    /// Returns the event name.
    #[inline]
    pub fn name(&self) -> String {
        match self {
            Self::ChallengeRequest(event) => event.name(),
            Self::ChallengeResponse(event) => event.name(),
            Self::Disconnect(event) => event.name(),
            Self::Ping(event) => event.name(),
            Self::WorkerBatch(event) => event.name(),
        }
    }

    /// Returns the event ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::ChallengeRequest(..) => 0,
            Self::ChallengeResponse(..) => 1,
            Self::Disconnect(..) => 2,
            Self::Ping(..) => 3,
            Self::WorkerBatch(..) => 4,
        }
    }

    /// Serializes the event into the buffer.
    #[inline]
    pub fn serialize<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(&self.id().to_le_bytes()[..])?;

        match self {
            Self::ChallengeRequest(event) => event.serialize(writer),
            Self::ChallengeResponse(event) => event.serialize(writer),
            Self::Disconnect(event) => event.serialize(writer),
            Self::Ping(event) => event.serialize(writer),
            Self::WorkerBatch(event) => event.serialize(writer),
        }
    }

    /// Deserializes the given buffer into a event.
    #[inline]
    pub fn deserialize(mut bytes: BytesMut) -> Result<Self> {
        // Ensure there is at least a event ID in the buffer.
        if bytes.remaining() < 2 {
            bail!("Missing event ID");
        }

        // Read the event ID.
        let id: u16 = bytes.get_u16_le();

        // Deserialize the data field.
        let event = match id {
            0 => Self::ChallengeRequest(EventTrait::deserialize(bytes)?),
            1 => Self::ChallengeResponse(EventTrait::deserialize(bytes)?),
            2 => Self::Disconnect(EventTrait::deserialize(bytes)?),
            3 => Self::Ping(EventTrait::deserialize(bytes)?),
            4 => Self::WorkerBatch(EventTrait::deserialize(bytes)?),
            _ => bail!("Unknown event ID {id}"),
        };

        Ok(event)
    }
}
