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

use rand::Rng;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::*;

pub const LOCALHOST: &'static str = "0.0.0.0";

/// Returns a random tcp socket address
pub fn random_socket_address() -> SocketAddr {
    let mut rng = rand::thread_rng();
    let string = format!("{}:{}", LOCALHOST, rng.gen_range(3333, 9999));
    string.parse::<SocketAddr>().unwrap()
}

pub struct TcpServer {
    address: SocketAddr,
}

impl TcpServer {
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }

    pub fn new_random() -> Self {
        Self {
            address: random_socket_address(),
        }
    }

    pub async fn listen(&self, should_reject: bool) -> anyhow::Result<()> {
        debug!("Starting listener at {:?}...", self.address);
        let listener = TcpListener::bind(&self.address).await?;
        info!("Listening at {:?}", self.address);

        if !should_reject {
            loop {
                let inbound = listener.accept().await;
                info!("{:?}", inbound);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::{io::AsyncWriteExt, net::TcpStream};

    #[tokio::test]
    async fn test_listen() {
        let remote_address = random_socket_address();

        // Start a TcpServer.
        tokio::task::spawn(async move {
            let server = TcpServer::new(remote_address);
            server.listen(false).await.unwrap();
        });

        // Connect to the TcpServer.
        let mut channel = TcpStream::connect(remote_address).await.unwrap();

        // Send a message.
        let result = channel.write_all(b"hello").await;
        assert!(result.is_ok());
    }
}
