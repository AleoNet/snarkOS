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

use crate::Vec;
use core::marker::PhantomData;
use digest::{generic_array::GenericArray, Digest};
use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use snarkos_utilities::bytes::{FromBytes, ToBytes};

/// A `SeedableRng` that refreshes its seed by hashing together the previous seed
/// and the new seed material.
// TODO: later: re-evaluate decision about ChaChaRng
pub struct FiatShamirRng<D: Digest> {
    r: ChaChaRng,
    seed: GenericArray<u8, D::OutputSize>,
    #[doc(hidden)]
    digest: PhantomData<D>,
}

impl<D: Digest> RngCore for FiatShamirRng<D> {
    #[inline]
    fn next_u32(&mut self) -> u32 {
        self.r.next_u32()
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.r.next_u64()
    }

    #[inline]
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.r.fill_bytes(dest);
    }

    #[inline]
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        Ok(self.r.fill_bytes(dest))
    }
}

impl<D: Digest> FiatShamirRng<D> {
    /// Create a new `Self` by initializing with a fresh seed.
    /// `self.seed = H(self.seed || new_seed)`.
    #[inline]
    pub fn from_seed<'a, T: 'a + ToBytes>(seed: &'a T) -> Self {
        let mut bytes = Vec::new();
        seed.write(&mut bytes).expect("failed to convert to bytes");
        let seed = D::digest(&bytes);
        let r_seed: [u8; 32] = FromBytes::read(seed.as_ref()).expect("failed to get [u32; 8]");
        let r = ChaChaRng::from_seed(r_seed);
        Self {
            r,
            seed,
            digest: PhantomData,
        }
    }

    /// Refresh `self.seed` with new material. Achieved by setting
    /// `self.seed = H(self.seed || new_seed)`.
    #[inline]
    pub fn absorb<'a, T: 'a + ToBytes>(&mut self, seed: &'a T) {
        let mut bytes = Vec::new();
        seed.write(&mut bytes).expect("failed to convert to bytes");
        bytes.extend_from_slice(&self.seed);
        self.seed = D::digest(&bytes);
        let seed: [u8; 32] = FromBytes::read(self.seed.as_ref()).expect("failed to get [u32; 8]");
        self.r = ChaChaRng::from_seed(seed);
    }
}
