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

/// Returns a random tcp socket address and binds it to a listener
pub async fn random_bound_address() -> (SocketAddr, TcpListener) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    (addr, listener)
}

pub struct TcpServer {
    pub address: SocketAddr,
    listener: Option<TcpListener>,
}

impl TcpServer {
    pub async fn new() -> Self {
        let (address, listener) = random_bound_address().await;

        Self {
            address,
            listener: Some(listener),
        }
    }

    pub async fn listen(&mut self, should_reject: bool) -> anyhow::Result<()> {
        let listener = self.listener.take().unwrap();

        if !should_reject {
            tokio::spawn(async move {
                loop {
                    let inbound = listener.accept().await;
                    println!("{:?}", inbound);
                }
            });
        }
        // gets dropped otherwise

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::{io::AsyncWriteExt, net::TcpStream};

    #[tokio::test]
    async fn test_listen() {
        let remote_address = random_bound_address();

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
