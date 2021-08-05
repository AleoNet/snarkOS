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

use std::{any::Any, fmt};

use snarkvm_dpc::{testnet1::instantiated::Components, Address as AccountAddress, PrivateKey as AccountPrivateKey};
use snarkvm_utilities::{FromBytes, ToBytes};

use crate::Digest;

pub trait AddressContainer: ToBytes + FromBytes + Send + Sync {}

// needs basedpccomponents to impl sync + send
// impl<B: BaseDPCComponents> AddressContainer for AccountAddress<B> {

// }

impl AddressContainer for AccountAddress<Components> {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address(pub Digest);

impl<T: AddressContainer> From<T> for Address {
    fn from(other: T) -> Self {
        let mut out = Digest::default();
        other.write_le(&mut out.0).expect("failed to write address");

        Address(out)
    }
}

impl Address {
    pub fn into<T: AddressContainer>(self) -> T {
        T::read_le(&mut &self.0.0[..]).expect("illegal cross-network address contamination")
    }
}

pub trait PrivateKeyContainer: Any + Send + Sync {}

impl PrivateKeyContainer for AccountPrivateKey<Components> {}

pub struct PrivateKey {
    inner: Box<dyn Any + Send + Sync + 'static>,
}

impl<T: PrivateKeyContainer> From<T> for PrivateKey {
    fn from(other: T) -> Self {
        PrivateKey { inner: Box::new(other) }
    }
}

impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PrivateKey")
    }
}

impl PrivateKey {
    pub fn into<T: PrivateKeyContainer>(self) -> T {
        *self
            .inner
            .downcast()
            .expect("illegal cross-network private key contamination")
    }

    pub fn into_ref<T: PrivateKeyContainer>(&self) -> &T {
        *self
            .inner
            .downcast_ref()
            .expect("illegal cross-network private key contamination")
    }
}
