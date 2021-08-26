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

use std::time::Duration;

use tokio::{net::TcpListener, task};

use snarkos_metrics::{self as metrics, connections};

use crate::{errors::NetworkError, Node};

impl Node {
    /// This method handles new inbound connection requests.
    pub async fn listen(&self) -> Result<(), NetworkError> {
        let listener = TcpListener::bind(&self.config.desired_address).await?;
        let own_listener_address = listener.local_addr()?;

        self.set_local_address(own_listener_address);
        info!("Initializing listener for node ({:x})", self.id);

        let node_clone = self.clone();
        let listener_handle = task::spawn(async move {
            info!("Listening for nodes at {}", own_listener_address);

            loop {
                match listener.accept().await {
                    Ok((stream, remote_address)) => {
                        if !node_clone.can_connect() {
                            continue;
                        }
                        let node_clone = node_clone.clone();
                        tokio::spawn(async move {
                            match node_clone
                                .peer_book
                                .receive_connection(node_clone.clone(), remote_address, stream)
                            {
                                Ok(_) => (),
                                Err(e) => {
                                    error!("Failed to receive a connection: {}", e);
                                }
                            }
                        });

                        // add a tiny delay to avoid connecting above the limit
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(e) => error!("Failed to accept a connection: {}", e),
                }
                metrics::increment_counter!(connections::ALL_ACCEPTED);
            }
        });

        self.register_task(listener_handle);

        Ok(())
    }
}
