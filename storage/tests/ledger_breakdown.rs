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

use std::{collections::HashMap, fmt, mem};

use snarkos_environment::CurrentNetwork;
use snarkos_storage::{
    storage::{
        rocksdb::{RocksDB, PREFIX_LEN},
        MapId,
        Storage,
    },
    LedgerState,
};

struct PrefixInfo {
    id: MapId,
    num_records: usize,
    size_of_keys: usize,
    size_of_values: usize,
}

impl PrefixInfo {
    fn new(prefix: u16) -> Self {
        Self {
            id: MapId::from(prefix),
            num_records: 0,
            size_of_keys: 0,
            size_of_values: 0,
        }
    }
}

pub fn display_bytes(bytes: f64) -> String {
    const GB: f64 = 1_000_000_000.0;
    const MB: f64 = 1_000_000.0;
    const KB: f64 = 1_000.0;

    if bytes >= GB {
        format!("{:.2} GB", bytes / GB)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.2} kB", bytes / KB)
    } else {
        format!("{:.2} B", bytes)
    }
}

impl fmt::Display for PrefixInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{:?}:", self.id)?;
        writeln!(f, "    number of records: {}", self.num_records)?;
        writeln!(f, "    size of keys: {}", display_bytes(self.size_of_keys as f64))?;
        writeln!(f, "    size of values: {}", display_bytes(self.size_of_values as f64))
    }
}

// This test processes a ledger containing 1000 blocks and displays the breakdown of its contents.
#[test]
#[ignore = "This test is purely informative."]
fn show_ledger_breakdown() {
    let temp_dir = tempfile::tempdir().expect("Failed to open temporary directory").into_path();
    // Create an empty ledger.
    let ledger: LedgerState<CurrentNetwork> =
        LedgerState::open_writer_with_increment::<RocksDB, _>(&temp_dir, 1).expect("Failed to initialize ledger");
    // Import a dump of a ledger containing 1k blocks.
    ledger
        .storage()
        .import("benches/storage_1k_blocks")
        .expect("Couldn't import the test ledger");

    let rocksdb = ledger.storage();
    let mut iterator = rocksdb.inner().raw_iterator();
    iterator.seek_to_first();
    let map_prefix_len = mem::size_of::<MapId>();
    let common_prefix_len = PREFIX_LEN - map_prefix_len;

    let mut prefix_infos: HashMap<u16, PrefixInfo> = Default::default();

    while iterator.valid() {
        if let (Some(key), Some(value)) = (iterator.key(), iterator.value()) {
            let prefix = u16::from_le_bytes(key[common_prefix_len..][..map_prefix_len].try_into().unwrap());
            let prefix_info = prefix_infos.entry(prefix).or_insert(PrefixInfo::new(prefix));
            prefix_info.num_records += 1;
            prefix_info.size_of_keys += key.len();
            prefix_info.size_of_values += value.len();
        }
        iterator.next();
    }

    let (mut num_records, mut size_of_keys, mut size_of_values) = (0, 0, 0);

    for prefix_info in prefix_infos.values() {
        num_records += prefix_info.num_records;
        size_of_keys += prefix_info.size_of_keys;
        size_of_values += prefix_info.size_of_values;
    }

    println!("number of all records: {}", num_records);
    println!("size of all records: {}", display_bytes((size_of_keys + size_of_values) as f64));
    println!("size of all keys: {}", display_bytes(size_of_keys as f64));
    println!("size of all values: {}\n", display_bytes(size_of_values as f64));

    let mut sorted_infos: Vec<_> = prefix_infos.into_iter().collect();
    sorted_infos.sort_unstable_by_key(|(id, _)| *id);

    for (_id, prefix_info) in sorted_infos {
        println!("{}", prefix_info);
    }
}
