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

#![forbid(unsafe_code)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate tracing;

pub mod helpers;

mod event;
pub use event::*;

mod gateway;
pub use gateway::*;

mod primary;
pub use primary::*;

mod shared;
pub use shared::*;

mod worker;
pub use worker::*;

pub const CONTEXT: &str = "[MemoryPool]";
pub const MAX_COMMITTEE_SIZE: u16 = 128;
pub const MAX_WORKERS: u8 = 10;
pub const MEMORY_POOL_PORT: u16 = 5000;
pub const WORKER_PING_INTERVAL: u64 = 1000; // ms

// TODO (howardwu): Switch the worker's `EntryID` to use or include a sha256/blake2s hash.
// TODO (howardwu): Implement sha256/blake2s hashing on `Data::Bytes`, so we can compare IDs without deserializing.
//  This is needed by the worker in `process_event_response` to guarantee integrity of the entry.
