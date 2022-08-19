// Copyright (C) 2019-2022 Aleo Systems Inc.
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

#[macro_use]
extern crate tracing;

mod ledger;
pub use ledger::*;

mod network;
pub use network::*;

mod store;
pub use store::*;

use backoff::{future::retry, ExponentialBackoff};
use futures::Future;
use std::time::Duration;

pub(crate) async fn handle_dispatch_error<'a, T, F>(func: impl Fn() -> F + 'a) -> reqwest::Result<T>
where
    F: Future<Output = Result<T, reqwest::Error>>,
{
    retry(default_backoff(), || async { func().await.map_err(from_reqwest_err) }).await
}

fn from_reqwest_err(err: reqwest::Error) -> backoff::Error<reqwest::Error> {
    use backoff::Error;

    if err.is_timeout() {
        debug!("Retrying server timeout error");
        Error::Transient { err, retry_after: None }
    } else {
        Error::Permanent(err)
    }
}

fn default_backoff() -> ExponentialBackoff {
    ExponentialBackoff {
        max_interval: Duration::from_secs(10),
        max_elapsed_time: Some(Duration::from_secs(45)),
        ..Default::default()
    }
}
