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

use super::*;

use snarkvm::{
    ledger::narwhal::Data,
    prelude::{FromBytes, ToBytes},
};

use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnconfirmedTransaction<N: Network> {
    pub transaction_id: N::TransactionID,
    pub transaction: Data<Transaction<N>>,
}

impl<N: Network> From<Transaction<N>> for UnconfirmedTransaction<N> {
    /// Initializes a new `UnconfirmedTransaction` message.
    fn from(transaction: Transaction<N>) -> Self {
        Self { transaction_id: transaction.id(), transaction: Data::Object(transaction) }
    }
}

impl<N: Network> MessageTrait for UnconfirmedTransaction<N> {
    /// Returns the message name.
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        "UnconfirmedTransaction".into()
    }
}

impl<N: Network> ToBytes for UnconfirmedTransaction<N> {
    fn write_le<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        self.transaction_id.write_le(&mut writer)?;
        self.transaction.write_le(&mut writer)?;
        Ok(())
    }
}

impl<N: Network> FromBytes for UnconfirmedTransaction<N> {
    fn read_le<R: io::Read>(mut reader: R) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(Self { transaction_id: N::TransactionID::read_le(&mut reader)?, transaction: Data::read_le(reader)? })
    }
}
