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
    pub fn into<T2: From<Data<T>> + From<T> + FromBytes + ToBytes + Send + 'static>(self) -> Data<T2> {
        match self {
            Self::Object(x) => Data::Object(x.into()),
            Self::Buffer(bytes) => Data::Buffer(bytes),
        }
    }

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
