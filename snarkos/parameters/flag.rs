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

// Global

pub const NO_JSONRPC: &str = "[no-jsonrpc] --no-jsonrpc 'Run the node without running the json rpc server'";

pub const IS_BOOTNODE: &str =
    "[is-bootnode] --is-bootnode 'Run the node as a bootnode (IP is hard coded in the protocol)'";

pub const IS_MINER: &str = "[is-miner] --is-miner 'Start mining blocks from this node'";

pub const LIST: &str = "[list] -l --list 'List all available releases of snarkOS'";

pub const TRIM_STORAGE: &str = "[trim-storage] --trim-storage 'Remove non-canon items from the node's storage'";

pub const VALIDATE_STORAGE: &str = "[validate-storage] --validate-storage 'Check the integrity of the node's storage and attempt to fix encountered issues'";
