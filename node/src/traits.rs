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

use snarkos_node_executor::{Executor, NodeType};
use snarkos_node_router::Router;
use snarkvm::prelude::{Address, Network, PrivateKey, ViewKey};

pub trait NodeInterface<N: Network>: Executor {
    /// Returns the node type.
    fn node_type(&self) -> NodeType;

    /// Returns the node router.
    fn router(&self) -> &Router<N>;

    /// Returns the account private key of the node.
    fn private_key(&self) -> &PrivateKey<N>;

    /// Returns the account view key of the node.
    fn view_key(&self) -> &ViewKey<N>;

    /// Returns the account address of the node.
    fn address(&self) -> &Address<N>;
}
