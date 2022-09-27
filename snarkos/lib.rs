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

#![forbid(unsafe_code)]
#![allow(clippy::module_inception)]
#![allow(clippy::suspicious_else_formatting)]
#![allow(clippy::type_complexity)]

#[macro_use]
extern crate thiserror;
#[macro_use]
extern crate tracing;

pub mod logger;

mod account;
pub use account::*;

mod cli;
pub use cli::*;

mod ledger;
pub use ledger::*;

mod network;
pub use network::*;

mod node;
pub use node::*;

mod store;
pub use store::*;

mod updater;
pub use updater::*;

pub use snarkos_environment as environment;

#[cfg(feature = "rpc")]
pub use snarkos_rpc as rpc;

pub use snarkvm::prelude::{Address, Network};

pub mod prelude {
    pub use crate::environment::*;

    #[cfg(feature = "rpc")]
    pub use crate::rpc::*;

    pub use snarkvm::prelude::{Address, Network};
}

use anyhow::anyhow;
use backoff::{future::retry, ExponentialBackoff};
use futures::Future;
use std::time::Duration;

pub(crate) async fn handle_dispatch_error<'a, T, F>(func: impl Fn() -> F + 'a) -> anyhow::Result<T>
where
    F: Future<Output = Result<T, anyhow::Error>>,
{
    fn default_backoff() -> ExponentialBackoff {
        ExponentialBackoff {
            max_interval: Duration::from_secs(10),
            max_elapsed_time: Some(Duration::from_secs(45)),
            ..Default::default()
        }
    }

    fn from_anyhow_err(err: anyhow::Error) -> backoff::Error<anyhow::Error> {
        use backoff::Error;

        if let Ok(err) = err.downcast::<reqwest::Error>() {
            if err.is_timeout() {
                debug!("Retrying server timeout error");
                Error::Transient {
                    err: err.into(),
                    retry_after: None,
                }
            } else {
                Error::Permanent(err.into())
            }
        } else {
            Error::Transient {
                err: anyhow!("Block parse error"),
                retry_after: None,
            }
        }
    }

    retry(default_backoff(), || async { func().await.map_err(from_anyhow_err) }).await
}
