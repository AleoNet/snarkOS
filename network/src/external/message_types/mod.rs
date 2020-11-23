// Copyright (C) 2019-2020 Aleo Systems Inc.
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

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/block.md"))]
pub mod block;
#[doc(inline)]
pub use block::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_block.md"))]
pub mod getblock;
#[doc(inline)]
pub use getblock::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_memory_pool.md"))]
pub mod getmemorypool;
#[doc(inline)]
pub use getmemorypool::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_peers.md"))]
pub mod getpeers;
#[doc(inline)]
pub use getpeers::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/get_sync.md"))]
pub mod getsync;
#[doc(inline)]
pub use getsync::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/memory_pool.md"))]
pub mod memorypool;
#[doc(inline)]
pub use memorypool::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/peers.md"))]
pub mod peers;
#[doc(inline)]
pub use peers::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/ping.md"))]
pub mod ping;
#[doc(inline)]
pub use ping::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/pong.md"))]
pub mod pong;
#[doc(inline)]
pub use pong::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/sync.md"))]
pub mod sync;
#[doc(inline)]
pub use sync::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/sync_block.md"))]
pub mod syncblock;
#[doc(inline)]
pub use syncblock::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/transaction.md"))]
pub mod transaction;
#[doc(inline)]
pub use transaction::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/verack.md"))]
pub mod verack;
#[doc(inline)]
pub use verack::*;

#[cfg_attr(nightly, doc(include = "../../../documentation/network_messages/version.md"))]
pub mod version;
#[doc(inline)]
pub use version::*;
