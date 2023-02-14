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

use narwhal_types::Batch;

// A test transaction validator.
#[derive(Default, Clone)]
pub struct TestTransactionValidator;

#[async_trait::async_trait]
impl narwhal_worker::TransactionValidator for TestTransactionValidator {
    type Error = anyhow::Error;

    fn validate(&self, _transaction: &[u8]) -> Result<(), Self::Error> {
        // TODO: come up with some useful validation criteria
        Ok(())
    }

    async fn validate_batch(&self, _batch: &Batch) -> Result<(), Self::Error> {
        Ok(())
    }
}
