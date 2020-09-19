use crate::{
    external::{Block, Transaction},
    internal::context::Context,
};
use snarkos_errors::network::SendError;

use std::{net::SocketAddr, sync::Arc};

/// Broadcast transaction to connected peers
pub async fn propagate_transaction(
    context: Arc<Context>,
    transaction_bytes: Vec<u8>,
    transaction_sender: SocketAddr,
) -> Result<(), SendError> {
    debug!("Propagating a transaction to peers");

    let peer_book = context.peer_book.read().await;
    let local_address = *context.local_address.read().await;
    let connections = context.connections.read().await;
    let mut num_peers = 0;

    for (socket, _) in &peer_book.get_connected() {
        if *socket != transaction_sender && *socket != local_address {
            if let Some(channel) = connections.get(socket) {
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

/// Broadcast block to connected peers
pub async fn propagate_block(
    context: Arc<Context>,
    block_bytes: Vec<u8>,
    block_miner: SocketAddr,
) -> Result<(), SendError> {
    debug!("Propagating a block to peers");

    let peer_book = context.peer_book.read().await;
    let local_address = *context.local_address.read().await;
    let connections = context.connections.read().await;
    let mut num_peers = 0;

    for (socket, _) in &peer_book.get_connected() {
        if *socket != block_miner && *socket != local_address {
            if let Some(channel) = connections.get(socket) {
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
