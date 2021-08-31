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

use crate::Ledger;

use anyhow::*;
use snarkos_storage::Digest;

/// This object only serves as a temporary replacement for the regular Ledger so that it can be sent to a blocking task.
pub(crate) struct DummyLedger;

impl Ledger for DummyLedger {
    fn extend(&mut self, _new_cms: &[Digest], _new_sns: &[Digest], _new_memos: &[Digest]) -> Result<Digest> {
        unimplemented!()
    }

    fn rollback(&mut self, _commitments: &[Digest], _serial_numbers: &[Digest], _memos: &[Digest]) -> Result<()> {
        unimplemented!()
    }

    fn clear(&mut self) {
        unimplemented!()
    }

    fn commitment_len(&self) -> usize {
        unimplemented!()
    }

    fn contains_commitment(&self, _commitment: &Digest) -> bool {
        unimplemented!()
    }

    fn commitment_index(&self, _commitment: &Digest) -> Option<usize> {
        unimplemented!()
    }

    fn contains_serial(&self, _serial: &Digest) -> bool {
        unimplemented!()
    }

    fn contains_memo(&self, _memo: &Digest) -> bool {
        unimplemented!()
    }

    fn validate_digest(&self, _digest: &Digest) -> bool {
        unimplemented!()
    }

    fn digest(&self) -> Digest {
        unimplemented!()
    }

    fn generate_proof(&self, _commitment: &Digest, _index: usize) -> Result<Vec<(Digest, Digest)>> {
        unimplemented!()
    }

    fn validate_ledger(&self) -> bool {
        unimplemented!()
    }

    fn requires_async_task(&self, _new_commitments_len: usize, _new_serial_numbers_len: usize) -> bool {
        unimplemented!()
    }
}
