// Copyright (C) 2019-2020 Aleo Systems Inc.
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

use hex::{FromHex, FromHexError, ToHex};
use serde::{Deserialize, Serialize};
use std::{
    cmp,
    fmt,
    hash::{Hash, Hasher},
    ops,
    str,
};

/// Returns a fixed size hash.
/// Used for restricting message header length.
macro_rules! fixed_hash {
    ($hash: ident, $size: expr) => {
        #[repr(C)]
        #[derive(Serialize, Deserialize)]
        pub struct $hash([u8; $size]);

        impl AsRef<$hash> for $hash {
            fn as_ref(&self) -> &$hash {
                self
            }
        }

        impl Default for $hash {
            fn default() -> Self {
                Self([0u8; 12])
            }
        }

        impl From<[u8; $size]> for $hash {
            fn from(hash: [u8; $size]) -> Self {
                Self(hash)
            }
        }

        impl From<$hash> for [u8; $size] {
            fn from(hash: $hash) -> Self {
                hash.0
            }
        }

        impl<'a> From<&'a [u8]> for $hash {
            fn from(bytes: &[u8]) -> Self {
                let mut result = [0u8; $size];
                result[..].clone_from_slice(&bytes[0..$size]);
                $hash(result)
            }
        }

        impl From<u8> for $hash {
            fn from(vector: u8) -> Self {
                let mut result = Self::default();
                result.0[0] = vector;
                result
            }
        }

        impl From<&'static str> for $hash {
            fn from(string: &'static str) -> Self {
                string.parse().unwrap()
            }
        }

        impl str::FromStr for $hash {
            type Err = FromHexError;

            fn from_str(string: &str) -> Result<Self, Self::Err> {
                let vector: Vec<u8> = Vec::from_hex(string)?;
                match vector.len() {
                    $size => {
                        let mut result = [0u8; $size];
                        result.copy_from_slice(&vector);
                        Ok($hash(result))
                    }
                    _ => Err(FromHexError::InvalidStringLength),
                }
            }
        }

        impl fmt::Debug for $hash {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(&self.0.encode_hex::<String>())
            }
        }

        impl fmt::Display for $hash {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str(&self.0.encode_hex::<String>())
            }
        }

        impl ops::Deref for $hash {
            type Target = [u8; $size];

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl ops::DerefMut for $hash {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl cmp::PartialEq for $hash {
            fn eq(&self, other: &Self) -> bool {
                let self_ref: &[u8] = &self.0;
                let other_ref: &[u8] = &other.0;
                self_ref == other_ref
            }
        }

        impl cmp::PartialOrd for $hash {
            fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
                let self_ref: &[u8] = &self.0;
                let other_ref: &[u8] = &other.0;
                self_ref.partial_cmp(other_ref)
            }
        }

        impl Hash for $hash {
            fn hash<H>(&self, state: &mut H)
            where
                H: Hasher,
            {
                state.write(&self.0);
                state.finish();
            }
        }

        impl Eq for $hash {}

        impl $hash {
            pub fn take(&self) -> [u8; $size] {
                self.0
            }

            pub fn size() -> usize {
                $size
            }

            pub fn is_zero(&self) -> bool {
                self.0.iter().all(|b| *b == 0)
            }
        }
    };
}

fixed_hash!(HASH96, 12);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let vec: [u8; 12] = [112, 105, 110, 103, 0, 0, 0, 0, 0, 0, 0, 0];
        println!("{:?}", HASH96::from(vec));
    }
}
