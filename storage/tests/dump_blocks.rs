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

use snarkos_environment::CurrentNetwork;
use snarkos_storage::{
    storage::{rocksdb::RocksDB, ReadOnly},
    LedgerState,
};
use snarkvm::dpc::traits::network::Network;

#[test]
#[ignore = "This can be run whenever a block dump is needed."]
fn dump_blocks() {
    // Compose the correct file path for the parameter file.
    let mut source_path = aleo_std::aleo_dir();
    source_path.push("storage");
    source_path.push(format!("ledger-{}", CurrentNetwork::NETWORK_ID));

    // The path to dump the blocks to.
    let target_path = "./blocks.dump";
    // The number of blocks to dump.
    let num_blocks = 10;

    let (ledger, _) = LedgerState::<CurrentNetwork, _>::open_reader::<RocksDB<ReadOnly>, _>(source_path).unwrap();
    ledger.dump_blocks(target_path, num_blocks).unwrap();
}
