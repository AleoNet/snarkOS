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

use crate::helpers::{Entry, EntryID};
use snarkvm::console::prelude::*;

use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct Ready<N: Network> {
    /// The map of `entry IDs` to `entries`.
    entries: Arc<RwLock<HashMap<EntryID<N>, Entry<N>>>>,
}

impl<N: Network> Default for Ready<N> {
    /// Initializes a new instance of the ready queue.
    fn default() -> Self {
        Self::new()
    }
}

impl<N: Network> Ready<N> {
    /// Initializes a new instance of the ready queue.
    pub fn new() -> Self {
        Self { entries: Default::default() }
    }

    /// Returns the number of entries in the ready queue.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Returns `true` if the ready queue contains the specified `entry ID`.
    pub fn contains(&self, entry_id: impl Into<EntryID<N>>) -> bool {
        self.entries.read().contains_key(&entry_id.into())
    }

    /// Returns the entry IDs.
    pub fn entry_ids(&self) -> Vec<EntryID<N>> {
        self.entries.read().keys().copied().collect()
    }

    /// Inserts the specified (`entry ID`, `entry`) to the ready queue.
    pub fn insert(&self, entry_id: impl Into<EntryID<N>>, entry: impl Into<Entry<N>>) {
        self.entries.write().insert(entry_id.into(), entry.into());
    }

    /// Removes the specified `entry ID` from the ready queue.
    pub fn remove(&self, entry_id: impl Into<EntryID<N>>) {
        self.entries.write().remove(&entry_id.into());
    }
}
