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

use crate::Environment;
use snarkvm::prelude::*;

use anyhow::{anyhow, Result};
use once_cell::sync::OnceCell;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};

#[derive(Clone, Debug)]
pub enum Message<N: Network> {
    /// ChallengeRequest := (listener_port, block_height)
    ChallengeRequest(u16, u32),
    /// ChallengeResponse := (block_header)
    ChallengeResponse(BlockHeader<N>),
    /// Ping := (block_height)
    Ping(u32),
    /// Pong := ()
    Pong,
}

impl<N: Network> Message<N> {
    /// Returns the message name.
    #[inline]
    pub fn name(&self) -> &str {
        match self {
            Self::ChallengeRequest(..) => "ChallengeRequest",
            Self::ChallengeResponse(..) => "ChallengeResponse",
            Self::Ping(..) => "Ping",
            Self::Pong => "Pong",
        }
    }

    /// Returns the message ID.
    #[inline]
    pub fn id(&self) -> u16 {
        match self {
            Self::ChallengeRequest(..) => 0,
            Self::ChallengeResponse(..) => 1,
            Self::Ping(..) => 2,
            Self::Pong => 3,
        }
    }

    /// Returns the message data as bytes.
    #[inline]
    pub fn data(&self) -> Result<Vec<u8>> {
        match self {
            Self::ChallengeRequest(listener_port, block_height) => {
                Ok([listener_port.to_le_bytes().to_vec(), block_height.to_le_bytes().to_vec()].concat())
            }
            Self::ChallengeResponse(block_header) => block_header.to_bytes_le(),
            Self::Ping(block_height) => Ok(block_height.to_le_bytes().to_vec()),
            Self::Pong => Ok(vec![]),
        }
    }

    /// Serializes the given message into bytes.
    #[inline]
    pub fn serialize(&self) -> Result<Vec<u8>> {
        Ok([self.id().to_le_bytes().to_vec(), self.data()?].concat())
    }

    /// Deserializes the given buffer into a message.
    #[inline]
    pub fn deserialize(buffer: &[u8]) -> Result<Self> {
        // Ensure the buffer contains at least the length of an ID.
        if buffer.len() < 2 {
            return Err(anyhow!("Invalid message buffer"));
        }

        // Split the buffer into the ID and data portion.
        let id = u16::from_le_bytes([buffer[0], buffer[1]]);
        let data = &buffer[2..];

        // Deserialize the data field.
        match id {
            0 => Ok(Self::ChallengeRequest(
                bincode::deserialize(&data[0..2])?,
                bincode::deserialize(&data[2..])?,
            )),
            1 => Ok(Self::ChallengeResponse(bincode::deserialize(data)?)),
            2 => Ok(Self::Ping(bincode::deserialize(data)?)),
            3 => match data.len() == 0 {
                true => Ok(Self::Pong),
                false => Err(anyhow!("Invalid 'Pong' message")),
            },
            _ => Err(anyhow!("Invalid message ID")),
        }
    }
}
