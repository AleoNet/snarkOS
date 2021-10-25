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

use crate::{network::peers::Peers, Environment, Message, Node, Status};
use snarkos_ledger::ledger::Ledger;
use snarkvm::dpc::prelude::*;
use crate::ledger::{LedgerRequest, LedgerRouter};

use std::net::SocketAddr;
use anyhow::{anyhow, Result};
use rand::thread_rng;
use std::{
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};
use tokio::{sync::RwLock, task};

pub(crate) struct Miner<N: Network> {
    miner_address: Address<N>,
}

impl<N: Network> Miner<N> {
    pub(crate) fn spawn<E: Environment>(recipient: Address<N>, ledger_router: LedgerRouter<N, E>) -> task::JoinHandle<()> {
        // task::spawn(async move {
        //     loop {
        //         // Retrieve the status of the node.
        //         let status = node.status();
        //         // Ensure the node is not syncing or shutting down.
        //         if status != Status::Syncing && status != Status::ShuttingDown {
        //             // Set the status of the node to mining.
        //             node.set_status(Status::Mining);
        //             // Start the mining process.
        //             let result = Miner::mine_next_block(node.ledger(), node.peers(), recipient, &node.terminator()).await;
        //             // Ensure the miner did not error.
        //             if let Err(error) = result {
        //                 // Sleep for 10 seconds.
        //                 tokio::time::sleep(Duration::from_secs(10)).await;
        //                 warn!("{}", error);
        //             }
        //         }
        //     }
        // })
    }
}
