use crate::{base::Message, Peer};
use snarkos_errors::network::SendError;
use snarkos_objects::BlockHeaderHash;

use chrono::{DateTime, Utc};
use std::{collections::HashMap, net::SocketAddr};

/// Send block to a peer
pub async fn send_block(peer_address: SocketAddr, block_serialized: Vec<u8>) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::Block {
        block_serialized: block_serialized.clone(),
    };
    info!("Sending block to: {:?}", peer_address);

    peer.send(&message).await?;

    Ok(())
}

/// Request a block from a peer
pub async fn send_block_request(peer_address: SocketAddr, block_hash: BlockHeaderHash) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::BlockRequest {
        block_hash: block_hash.clone(),
    };

    info!("Requesting block {:?} from {:?}", block_hash, peer_address);

    peer.send(&message).await?;

    Ok(())
}

/// Send our memory pool request to a peer
pub async fn send_memory_pool_request(peer_address: SocketAddr) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::MemoryPoolRequest;

    info!("Sending memory pool request to {:?}", peer_address);

    peer.send(&message).await?;

    Ok(())
}

/// Send our memory pool transactions to a peer
pub async fn send_memory_pool_response(
    peer_address: SocketAddr,
    memory_pool_transactions: Vec<Vec<u8>>,
) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::MemoryPoolResponse {
        memory_pool_transactions,
    };

    info!("Sending memory pool response to {:?}", peer_address);

    peer.send(&message).await?;

    Ok(())
}

pub async fn send_peers_request(peer_address: SocketAddr) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    info!("Sending peers request to : {:?}", peer_address);

    peer.send(&Message::PeersRequest).await?;

    Ok(())
}

pub async fn send_peers_response(
    peer_address: SocketAddr,
    addresses: HashMap<SocketAddr, DateTime<Utc>>,
) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    info!("Sending peer list to {:?}", peer_address);

    peer.send(&Message::PeersResponse { addresses }).await?;

    Ok(())
}

/// Send a pong message to a peer
pub async fn send_ping(peer_address: SocketAddr) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::Ping;

    //    info!("Sending ping to {:?} from {:?}", peer, local_addr);

    peer.send(&message).await?;

    Ok(())
}

/// Send a pong message to a peer
pub async fn send_pong(peer_address: SocketAddr) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::Pong;

    info!("Sending pong to {:?}", peer);

    peer.send(&message).await?;

    Ok(())
}

/// Send block to our own server
pub async fn send_propagate_block(address_server: SocketAddr, block_serialized: Vec<u8>) -> Result<(), SendError> {
    let peer = Peer::new(address_server);

    let message = Message::PropagateBlock { block_serialized };

    peer.send(&message).await?;

    Ok(())
}

/// Send block to a syncing peer
pub async fn send_sync_block(peer_address: SocketAddr, block_serialized: Vec<u8>) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::SyncBlock {
        block_serialized: block_serialized.clone(),
    };
    info!("Sending sync block to: {:?}", peer_address);

    peer.send(&message).await?;

    Ok(())
}

/// Send sync block request
pub async fn send_sync_request(
    peer_address: SocketAddr,
    block_locator_hashes: Vec<BlockHeaderHash>,
) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::SyncRequest { block_locator_hashes };

    info!("Sending sync request to {:?}", peer_address);

    peer.send(&message).await?;

    Ok(())
}

/// Send all of our blocks to a peer
pub async fn send_sync_response(peer_address: SocketAddr, block_hashes: Vec<BlockHeaderHash>) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    let message = Message::SyncResponse { block_hashes };

    info!("Sending sync response to {:?}", peer_address);

    peer.send(&message).await?;

    Ok(())
}

/// Send transaction to a socket address. May be local or remote
pub async fn send_transaction(address_receiver: SocketAddr, transaction_bytes: Vec<u8>) -> Result<(), SendError> {
    info!("Sending transaction to: {:?}", address_receiver);

    let peer = Peer::new(address_receiver);

    let message = Message::Transaction { transaction_bytes };

    peer.send(&message).await?;

    Ok(())
}

/// Create a new peer connection. Send a Version message and expect a Verack.
pub async fn handshake_request(height: u32, peer_address: SocketAddr) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    // Send a Version Message
    let message = Message::Version {
        version: 1,
        timestamp: Utc::now(),
        height,
        address_receiver: peer_address,
    };

    info!("Sending initial Version to: {:?}", peer_address);
    peer.send(&message).await?;

    Ok(())
}

/// Accept a new peer connection. Send a Verack immediately.
/// If this node is not in our list of peers, send a Version and expect a Verack.
pub async fn handshake_response(height: u32, peer_address: SocketAddr, new_peer: bool) -> Result<(), SendError> {
    let peer = Peer::new(peer_address);

    // Send a Verack Message back
    let mut message = Message::Verack;
    info!("Sending Verack to:     {:?}", peer_address);
    peer.send(&message).await?;

    // check peerlist
    if new_peer {
        // Send a Version Message back
        message = Message::Version {
            version: 1,
            timestamp: Utc::now(),
            height,
            address_receiver: peer_address,
        };
        info!("Sending Version to     {:?}", peer_address);
        peer.send(&message).await?;
    }

    Ok(())
}
