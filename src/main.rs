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

use snarkos::{Miner, Node};

use snarkvm::{
    dpc::{prelude::*, testnet2::Testnet2},
    prelude::*,
};

use ::rand::thread_rng;
use anyhow::Result;
use tokio::{net::TcpListener, task};

#[tokio::main]
async fn main() -> Result<()> {
    // let addr = env::args()
    //     .nth(1)
    //     .unwrap_or_else(|| "127.0.0.1:8080".to_string());
    //
    // let listener = TcpListener::bind(&addr).await?;
    // println!("Listening on: {}", addr);

    let account = Account::<Testnet2>::new(&mut thread_rng());

    let node = Node::<Testnet2, Miner>::new()?;
    node.start_miner(account.address());

    std::future::pending::<()>().await;
    Ok(())
}
