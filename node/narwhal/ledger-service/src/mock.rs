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

use crate::{fmt_id, LedgerService};
use snarkvm::{
    ledger::narwhal::{Transmission, TransmissionID},
    prelude::{Field, Network, Result},
};

use std::collections::HashMap;
use tracing::*;

/// A mock ledger service that always returns `false`.
#[derive(Default)]
pub struct MockLedgerService<N: Network> {
    transmissions: HashMap<TransmissionID<N>, Transmission<N>>,
}

impl<N: Network> MockLedgerService<N> {
    /// Initializes a new mock ledger service.
    pub fn new() -> MockLedgerService<N> {
        MockLedgerService { transmissions: Default::default() }
    }
}

impl<N: Network> LedgerService<N> for MockLedgerService<N> {
    /// Returns `false` for all queries.
    fn contains_certificate(&self, certificate_id: &Field<N>) -> Result<bool> {
        trace!("[MockLedgerService] Contains certificate ID {} - false", fmt_id(certificate_id));
        Ok(false)
    }

    /// Returns `false` for all queries.
    fn contains_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<bool> {
        trace!("[MockLedgerService] Contains transmission ID {} - false", fmt_id(transmission_id));
        Ok(self.transmissions.contains_key(transmission_id))
    }

    /// Returns the transmission for the given transmission ID, if it exists.
    fn get_transmission(&self, transmission_id: &TransmissionID<N>) -> Result<Option<Transmission<N>>> {
        trace!("[MockLedgerService] Get transmission ID {}", fmt_id(transmission_id));
        Ok(self.transmissions.get(transmission_id).cloned())
    }
}
