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

use crate::{
    environment::Environment,
    external::{Block, HandshakeStatus, Transaction, Verack},
};
use snarkos_consensus::{
    memory_pool::{Entry, MemoryPool},
    ConsensusParameters, MerkleTreeLedger,
};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_errors::network::SendError;
use snarkos_utilities::bytes::FromBytes;

use std::{
    net::SocketAddr,
    sync::{atomic::AtomicU64, Arc},
};
use tokio::{
    net::TcpStream,
    sync::{Mutex, RwLock},
    task,
};

/// A stateless component for handling outbound network traffic.
#[derive(Debug, Clone)]
pub struct SendHandler {
    /// A counter for the number of send requests the handler processes.
    send_request_count: Arc<AtomicU64>,
    /// A counter for the number of send requests that succeeded.
    send_success_count: Arc<AtomicU64>,
    /// A counter for the number of send requests that failed.
    send_failure_count: Arc<AtomicU64>,
}

impl SendHandler {
    /// Creates a new instance of a `SendHandler`.
    #[inline]
    pub fn new() -> Self {
        Self {
            send_request_count: Arc::new(AtomicU64::new(0)),
            send_success_count: Arc::new(AtomicU64::new(0)),
            send_failure_count: Arc::new(AtomicU64::new(0)),
        }
    }

    // TODO (howardwu): Implement getters to all counters.
    // /// Returns the number of requests the manager has processed.
    // #[inline]
    // pub async fn get_request_count(&self) -> u64 {
    //     let counter = self.send_request_count.clone();
    //     counter.clone().into_inner()
    // }

    /// Returns the nonce for a handshake with the given remote address, if it exists.
    #[inline]
    pub async fn get_handshake_nonce(&self, environment: &Environment, remote_address: &SocketAddr) -> Option<u64> {
        // Acquire the handshakes read lock.
        let handshakes = environment.handshakes().read().await;
        match handshakes.get(remote_address) {
            Some(handshake) => Some(handshake.nonce),
            _ => None,
        }
    }

    /// Returns the state of the handshake at a peer address.
    pub async fn get_state(&self, environment: &Environment, address: SocketAddr) -> Option<HandshakeStatus> {
        // Acquire the handshake read lock.
        let handshakes = environment.handshakes().read().await;
        match handshakes.get(&address) {
            Some(handshake) => Some(handshake.get_state()),
            None => None,
        }
    }

    /// Broadcast block to connected peers
    pub async fn propagate_block(
        &self,
        environment: Environment,
        block_bytes: Vec<u8>,
        block_miner: SocketAddr,
    ) -> Result<(), SendError> {
        debug!("Propagating a block to peers");

        let peer_manager = environment.peer_manager_read().await;
        let local_address = environment.local_address();
        let mut num_peers = 0u16;

        for (socket, _) in peer_manager.get_all_connected().await {
            if socket != block_miner && socket != *local_address {
                if let Some(channel) = peer_manager.get_channel(&socket).await {
                    match channel.write(&Block::new(block_bytes.clone())).await {
                        Ok(_) => num_peers += 1,
                        Err(error) => warn!(
                            "Failed to propagate block to peer {}. (error message: {})",
                            channel.address, error
                        ),
                    }
                }
            }
        }

        debug!("Block propagated to {} peers", num_peers);

        Ok(())
    }

    /// Verify a transaction, add it to the memory pool, propagate it to peers.
    pub async fn process_transaction_internal(
        &self,
        environment: &Environment,
        consensus: &ConsensusParameters,
        parameters: &PublicParameters<Components>,
        storage: &Arc<RwLock<MerkleTreeLedger>>,
        memory_pool: &Arc<Mutex<MemoryPool<Tx>>>,
        transaction_bytes: Vec<u8>,
        transaction_sender: SocketAddr,
    ) -> Result<(), SendError> {
        if let Ok(transaction) = Tx::read(&transaction_bytes[..]) {
            let mut memory_pool = memory_pool.lock().await;

            if !consensus.verify_transaction(parameters, &transaction, &*storage.read().await)? {
                error!("Received a transaction that was invalid");
                return Ok(());
            }

            if transaction.value_balance.is_negative() {
                error!("Received a transaction that was a coinbase transaction");
                return Ok(());
            }

            let entry = Entry::<Tx> {
                size_in_bytes: transaction_bytes.len(),
                transaction,
            };

            if let Ok(inserted) = memory_pool.insert(&*storage.read().await, entry) {
                if inserted.is_some() {
                    info!("Transaction added to memory pool.");
                    self.propagate_transaction(environment, transaction_bytes, transaction_sender)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Broadcast transaction to connected peers
    pub async fn propagate_transaction(
        &self,
        environment: &Environment,
        transaction_bytes: Vec<u8>,
        transaction_sender: SocketAddr,
    ) -> Result<(), SendError> {
        debug!("Propagating a transaction to peers");

        let peer_manager = environment.peer_manager_read().await;
        let local_address = *environment.local_address();
        let connections = environment.peer_manager_read().await;
        let mut num_peers = 0u16;

        for (socket, _) in peer_manager.get_all_connected().await {
            if socket != transaction_sender && socket != local_address {
                if let Some(channel) = connections.get_channel(&socket).await {
                    match channel.write(&Transaction::new(transaction_bytes.clone())).await {
                        Ok(_) => num_peers += 1,
                        Err(error) => warn!(
                            "Failed to propagate transaction to peer {}. (error message: {})",
                            channel.address, error
                        ),
                    }
                }
            }
        }

        debug!("Transaction propagated to {} peers", num_peers);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::{Channel, HandshakeStatus, Message};
    use snarkos_testing::network::random_socket_address;

    use serial_test::serial;
    use tokio::net::TcpListener;

    #[tokio::test]
    #[serial]
    async fn test_send_and_receive_handshake() {
        let local_address = random_socket_address();
        let remote_address = random_socket_address();

        // 1. Bind to remote address.
        let mut remote_listener = TcpListener::bind(remote_address).await.unwrap();
        let mut remote_manager = SendHandler::new();

        tokio::spawn(async move {
            let mut local_listener = TcpListener::bind(local_address).await.unwrap();
            let mut local_manager = SendHandler::new();

            // 2. Local node sends handshake request
            let local_version = Version::new(1u64, 0u32, remote_address, local_address);
            local_manager.send_connection_request(&local_version).await;

            // 5. Check local node handshake state
            let (reader, _) = local_listener.accept().await.unwrap();
            let channel = Channel::new_read_only(reader).unwrap();
            assert_eq!(
                HandshakeStatus::Waiting,
                local_manager.get_state(remote_address).await.unwrap()
            );

            // 6. Local node accepts handshake response
            let (_name, bytes) = channel.read().await.unwrap();
            let verack = Verack::deserialize(bytes).unwrap();
            local_manager.accept_response(remote_address, verack).await;
            assert_eq!(
                HandshakeStatus::Accepted,
                local_manager.get_state(remote_address).await.unwrap()
            );

            // 7. Local node receives handshake request
            let (_name, bytes) = channel.read().await.unwrap();
            let remote_version = Version::deserialize(bytes).unwrap();

            // 8. Local node sends handshake response
            local_manager.receive_request(remote_version, remote_address).await;
        });

        // 3. Remote node accepts Local node connection
        let (reader, _) = remote_listener.accept().await.unwrap();

        // 4. Remote node sends handshake response, handshake request
        let (handshake, _, _) = remote_manager
            .receive_connection_request(1u64, 0u32, local_address, reader)
            .await
            .unwrap();
        assert_eq!(
            HandshakeStatus::Waiting,
            remote_manager.get_state(local_address).await.unwrap()
        );

        // 9. Local node accepts handshake response
        let (_, bytes) = handshake.channel.read().await.unwrap();
        let verack = Verack::deserialize(bytes).unwrap();
        remote_manager.accept_response(local_address, verack).await;
        assert_eq!(
            HandshakeStatus::Accepted,
            remote_manager.get_state(local_address).await.unwrap()
        )
    }
}
