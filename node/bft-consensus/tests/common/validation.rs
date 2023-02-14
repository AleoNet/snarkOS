// Copyright (C) 2019-2023 Aleo Systems Inc.
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

use narwhal_types::Batch;

// A test transaction validator.
#[derive(Default, Clone)]
pub struct TestTransactionValidator;

impl narwhal_worker::TransactionValidator for TestTransactionValidator {
    type Error = anyhow::Error;

    fn validate(&self, _transaction: &[u8]) -> Result<(), Self::Error> {
        // TODO: come up with some useful validation criteria
        Ok(())
    }

    fn validate_batch(&self, _batch: &Batch) -> Result<(), Self::Error> {
        Ok(())
    }
}
