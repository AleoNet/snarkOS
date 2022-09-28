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

mod message;
pub use message::*;

mod peer;
pub use peer::*;

mod peers;
pub use peers::*;

use crate::{environment::Environment, Ledger};

use snarkvm::prelude::*;

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task,
};

/// Handle connection listener for new peers.
pub fn handle_listener<N: Network, E: Environment>(listener: TcpListener, ledger: Arc<Ledger<N, E>>) -> task::JoinHandle<()> {
    info!("Listening to connections at: {}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        loop {
            let ledger_clone = ledger.clone();

            match listener.accept().await {
                // Process the inbound connection request.
                Ok((stream, peer_ip)) => {
                    E::resources().register_task(
                        None,
                        tokio::spawn(
                            async move { Peer::handler(stream, peer_ip, ledger_clone.clone(), &ledger_clone.peers().router()).await },
                        ),
                    );
                }
                Err(error) => warn!("Failed to accept a connection: {}", error),
            }
        }
    })
}

// TODO (raychu86): Handle this request via `Message::BlockRequest`. This is currently not done,
//  because the node has not established the leader as a peer.
/// Request the genesis block from the leader.
pub(super) async fn request_genesis_block<N: Network>(leader_ip: IpAddr) -> Result<Block<N>> {
    info!("Requesting genesis block from {}", leader_ip);
    let block_string = reqwest::get(format!("http://{leader_ip}/testnet3/block/0")).await?.text().await?;

    Block::from_str(&block_string)
}

/// Send a ping to all peers every 10 seconds.
pub fn send_pings<N: Network, E: Environment>(ledger: Arc<Ledger<N, E>>) -> task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(10));
        loop {
            interval.tick().await;

            if let Err(err) = ledger
                .peers()
                .router()
                .send(PeersRequest::MessagePropagate(*ledger.peers().local_ip(), Message::<N>::Ping))
                .await
            {
                warn!("Error broadcasting Ping to peers: {}", err);
            }
        }
    })
}

/// Handle connection with the leader.
pub fn connect_to_leader<N: Network, E: Environment>(initial_peer: SocketAddr, ledger: Arc<Ledger<N, E>>) -> task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(time::Duration::from_secs(10));
        loop {
            if !ledger.peers().is_connected_to(&initial_peer).await {
                trace!("Attempting to connect to peer {}", initial_peer);
                match TcpStream::connect(initial_peer).await {
                    Ok(stream) => {
                        let ledger_clone = ledger.clone();
                        E::resources().register_task(
                            None,
                            tokio::spawn(async move {
                                Peer::handler(stream, initial_peer, ledger_clone.clone(), &ledger_clone.peers().router()).await;
                            }),
                        );
                    }
                    Err(error) => warn!("Failed to connect to peer {}: {}", initial_peer, error),
                }
            }
            interval.tick().await;
        }
    })
}
