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

use circular_queue::CircularQueue;

///
/// A helper struct to maintain a bounded number of elements in a map.
///
/// Note that this implementation can likely be improved significantly.
/// Due to the fact that the number of elements stored in the map is small,
/// the performance overhead should not be significant.
///
#[derive(Clone, Debug)]
pub struct CircularMap<K: Clone + PartialEq, V: Clone, const N: u32> {
    queue: CircularQueue<(K, V)>,
}

impl<K: Clone + PartialEq, V: Clone, const N: u32> CircularMap<K, V, N> {
    ///
    /// Initializes a new instance of a circular map, of pre-defined size.
    ///
    pub fn new() -> Self {
        Self {
            queue: CircularQueue::with_capacity(N as usize),
        }
    }

    ///
    /// Returns `true` if the circular map is empty.
    ///
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    ///
    /// Returns the number of key-value pairs in the circular map.
    ///
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    ///
    /// Returns the maximum capacity of the circular map.
    ///
    pub fn capacity(&self) -> usize {
        self.queue.capacity()
    }

    ///
    /// Returns `true` if the given key exists in the circular map.
    ///
    pub fn contains_key(&self, key: &K) -> bool {
        self.queue.iter().any(|(k, _)| k == key)
    }

    ///
    /// Returns the value for the given key from the map, if it exists.
    ///
    pub fn get(&self, key: &K) -> Option<&V> {
        match self.queue.iter().find(|(k, _)| k == key) {
            Some((_, value)) => Some(value),
            None => None,
        }
    }

    ///
    /// Inserts the given key-value pair into the circular map, returning a `bool`
    /// indicating whether the insertion took place.
    ///
    pub fn insert(&mut self, key: K, value: V) -> bool {
        if !self.contains_key(&key) {
            self.queue.push((key, value));
            true
        } else {
            false
        }
    }

    ///
    /// Removes the key-value pair for the given key from the circular map.
    ///
    pub fn remove(&mut self, key: &K) {
        let mut queue = CircularQueue::with_capacity(N as usize);
        for (k, v) in self.queue.iter() {
            if key != k {
                queue.push((k.clone(), v.clone()));
            }
        }
        self.queue = queue;
    }
}

impl<K: Clone + PartialEq, V: Clone, const N: u32> Default for CircularMap<K, V, N> {
    fn default() -> Self {
        Self::new()
    }
}
