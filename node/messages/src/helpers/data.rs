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

use snarkvm::prelude::{FromBytes, ToBytes};

use ::bytes::Bytes;
use anyhow::Result;
use std::io::Write;
use tokio::task;

/// This object enables deferred deserialization / ahead-of-time serialization for objects that
/// take a while to deserialize / serialize, in order to allow these operations to be non-blocking.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Data<T: FromBytes + ToBytes + Send + 'static> {
    Object(T),
    Buffer(Bytes),
}

impl<T: FromBytes + ToBytes + Send + 'static> Data<T> {
    pub async fn deserialize(self) -> Result<T> {
        match self {
            Self::Object(x) => Ok(x),
            Self::Buffer(bytes) => match task::spawn_blocking(move || T::from_bytes_le(&bytes)).await {
                Ok(x) => x,
                Err(err) => Err(err.into()),
            },
        }
    }

    pub fn deserialize_blocking(self) -> Result<T> {
        match self {
            Self::Object(x) => Ok(x),
            Self::Buffer(bytes) => T::from_bytes_le(&bytes),
        }
    }

    pub async fn serialize(self) -> Result<Bytes> {
        match self {
            Self::Object(x) => match task::spawn_blocking(move || x.to_bytes_le()).await {
                Ok(bytes) => bytes.map(|vec| vec.into()),
                Err(err) => Err(err.into()),
            },
            Self::Buffer(bytes) => Ok(bytes),
        }
    }

    pub fn serialize_blocking_into<W: Write>(&self, writer: &mut W) -> Result<()> {
        match self {
            Self::Object(x) => {
                let bytes = x.to_bytes_le()?;
                Ok(writer.write_all(&bytes)?)
            }
            Self::Buffer(bytes) => Ok(writer.write_all(bytes)?),
        }
    }
}
