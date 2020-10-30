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

use crate::external::message_types::*;

use std::{fmt, net::SocketAddr};

pub type Receiver = SocketAddr;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Request {
    Block(Receiver, Block),
    GetPeers(Receiver, GetPeers),
    Transaction(Receiver, Transaction),
    Verack(Verack),
    Version(Version),
}

impl Request {
    pub fn name(&self) -> &str {
        match self {
            Request::Block(_, _) => "Block",
            Request::GetPeers(_, _) => "GetPeers",
            Request::Transaction(_, _) => "Transaction",
            Request::Verack(_) => "Verack",
            Request::Version(_) => "Version",
        }
    }

    pub fn receiver(&self) -> Receiver {
        match self {
            Request::Block(receiver, _) => *receiver,
            Request::GetPeers(receiver, _) => *receiver,
            Request::Transaction(receiver, _) => *receiver,
            Request::Verack(verack) => verack.receiver,
            Request::Version(version) => version.receiver,
        }
    }

    // pub fn payload(&self) -> Box<&dyn Message> {
    //     match self {
    //         Request::Block(_, payload) => payload,
    //         Request::GetPeers(_, payload) => payload,
    //         Request::Transaction(_, payload) => payload,
    //         Request::Verack(payload) => payload,
    //         Request::Version(payload) => payload,
    //     }
    // }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}
