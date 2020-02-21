use snarkos_errors::network::SendError;

use crate::message::{
    types::{Verack, Version},
    Channel,
};
use chrono::Utc;
use std::{net::SocketAddr, sync::Arc};

/// Create a new peer connection. Send a Version message and expect a Verack.
pub async fn handshake_request(
    channel: Arc<Channel>,
    version: u64,
    height: u32,
    address_sender: SocketAddr,
) -> Result<(), SendError> {
    info!("Sending initial Version to: {:?}", channel.address);

    channel
        .write(&Version {
            version,
            timestamp: Utc::now(),
            height,
            address_receiver: channel.address,
            address_sender,
        })
        .await?;

    Ok(())
}

/// Accept a new peer connection. Send a Verack immediately.
/// If this node is not in our list of peers, send a Version and expect a Verack.
pub async fn handshake_response(
    channel: Arc<Channel>,
    new_peer: bool,
    version: u64,
    height: u32,
    address_sender: SocketAddr,
) -> Result<(), SendError> {
    info!("Sending Verack to:     {:?}", channel.address);

    channel.write(&Verack).await?;

    if new_peer {
        info!("Sending response Version to: {:?}", channel.address);
        channel
            .write(&Version {
                version,
                timestamp: Utc::now(),
                height,
                address_receiver: channel.address,
                address_sender,
            })
            .await?;
    }

    Ok(())
}
