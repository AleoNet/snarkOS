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

use std::{fmt, io, ops::Deref};

use serde::Serialize;
use smallvec::SmallVec;
use snarkvm_utilities::{ToBytes, Write};

/// `SmallVec` provides us with stack allocation in general cases but will fall back to heap for sizes > 64.
type InnerType = SmallVec<[u8; 64]>;

/// A generic storage for small-size binary blobs, generally digests.
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct Digest(pub InnerType);

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0[..]))
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <Self as fmt::Display>::fmt(self, f)
    }
}

impl Serialize for Digest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl Deref for Digest {
    type Target = SmallVec<[u8; 64]>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[allow(clippy::from_over_into)]
impl Into<InnerType> for Digest {
    fn into(self) -> InnerType {
        self.0
    }
}

impl<'a> From<&'a [u8]> for Digest {
    fn from(other: &'a [u8]) -> Self {
        Self(other.into())
    }
}

impl From<InnerType> for Digest {
    fn from(other: InnerType) -> Self {
        Self(other)
    }
}

impl<const N: usize> From<[u8; N]> for Digest {
    fn from(other: [u8; N]) -> Self {
        Self(other[..].into())
    }
}

impl AsRef<[u8]> for Digest {
    fn as_ref(&self) -> &[u8] {
        &self[..]
    }
}

impl ToBytes for Digest {
    fn write_le<W: Write>(&self, mut writer: W) -> io::Result<()> {
        writer.write_all(&self.0[..])
    }
}

impl Digest {
    pub fn bytes<const N: usize>(&self) -> Option<[u8; N]> {
        if self.len() == N {
            let mut out = [0u8; N];
            out.copy_from_slice(&self[..]);
            Some(out)
        } else {
            None
        }
    }
}
