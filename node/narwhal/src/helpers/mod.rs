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

pub mod cache;
pub use cache::*;

pub mod channels;
pub use channels::*;

pub mod dag;
pub use dag::*;

pub mod partition;
pub use partition::*;

pub mod pending;
pub use pending::*;

pub mod proposal;
pub use proposal::*;

pub mod ready;
pub use ready::*;

pub mod resolver;
pub use resolver::*;

pub mod storage;
pub use storage::*;

pub mod timestamp;
pub use timestamp::*;

/// Formats an ID into a truncated identifier (for logging purposes).
pub fn fmt_id(id: impl ToString) -> String {
    let id = id.to_string();
    let mut formatted_id = id.chars().take(16).collect::<String>();
    if id.chars().count() > 16 {
        formatted_id.push_str("..");
    }
    formatted_id
}

/// Returns the value that occurs most frequently in the given list.
pub fn majority_value<T: Default + Eq + std::hash::Hash + Clone>(list: impl Iterator<Item = T>) -> Option<T> {
    let mut occurrences = std::collections::HashMap::new();

    for value in list {
        let count = occurrences.entry(value.clone()).or_insert(0);
        *count += 1;
    }

    occurrences.into_iter().max_by_key(|&(_, count)| count).map(|(value, _)| value)
}
