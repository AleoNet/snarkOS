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
    external::{message::*, Peer, Verack, Version},
    peers::{PeerBook, PeerInfo},
    ConnReader,
    ConnWriter,
    Environment,
    Inbound,
    NetworkError,
    Outbound,
};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use parking_lot::RwLock;
use tokio::net::TcpStream;

/// A stateful component for managing the peer connections of this node server.
#[derive(Clone)]
pub struct Peers {
    /// The parameters and settings of this node server.
    pub(crate) environment: Environment,
    /// The inbound service of this node server.
    inbound: Arc<Inbound>,
    /// The outbound service of this node server.
    outbound: Arc<Outbound>,
    /// The list of connected and disconnected peers of this node server.
    peer_book: Arc<RwLock<PeerBook>>,
}

impl Peers {
    ///
    /// Creates a new instance of `Peers`.
    ///
    pub fn new(environment: Environment, inbound: Arc<Inbound>, outbound: Arc<Outbound>) -> Result<Self, NetworkError> {
        trace!("Instantiating the peer manager");

        // Load the peer book from storage, or create a new peer book.
        let peer_book = PeerBook::default();
        // let peer_book = match PeerBook::load(&*environment.storage_read().await) {
        //     // Case 1 - The peer book was found in storage.
        //     Ok(peer_book) => peer_book,
        //     // Case 2 - Either the peer book does not exist in storage, or could not be deserialized.
        //     // Create a new instance of the peer book.
        //     _ => PeerBook::new(*environment.local_address()),
        // };

        // Instantiate the peer manager.
        let peers = Self {
            environment,
            inbound,
            outbound,
            peer_book: Arc::new(RwLock::new(peer_book)),
        };

        // Save the peer book to storage.
        // peers.save_peer_book_to_storage().await?;

        Ok(peers)
    }

    ///
    /// Broadcasts updates with connected peers and maintains a permitted number of connected peers.
    ///
    pub async fn update(&self) -> Result<(), NetworkError> {
        // Fetch the number of connected peers.
        let number_of_connected_peers = self.number_of_connected_peers();
        trace!(
            "Connected to {} peer{}",
            number_of_connected_peers,
            if number_of_connected_peers == 1 { "" } else { "s" }
        );

        // Check that this node is not a bootnode.
        if !self.environment.is_bootnode() {
            // Check if this node server is below the permitted number of connected peers.
            if number_of_connected_peers < self.environment.minimum_number_of_connected_peers() {
                // Attempt to connect to the default bootnodes of the network.
                self.connect_to_bootnodes().await;

                // Attempt to connect to each disconnected peer saved in the peer book.
                self.connect_to_disconnected_peers().await;

                // Broadcast a `GetPeers` message to request for more peers.
                self.broadcast_getpeers_requests();
            }
        }

        // Check if this node server is above the permitted number of connected peers.
        if number_of_connected_peers > self.environment.maximum_number_of_connected_peers() {
            let number_to_disconnect = number_of_connected_peers - self.environment.maximum_number_of_connected_peers();
            trace!(
                "Disconnecting from the most recent {} peers to maintain their permitted number",
                number_to_disconnect
            );

            let mut connected = self
                .connected_peers()
                .into_iter()
                .map(|(_, peer_info)| peer_info)
                .collect::<Vec<_>>();
            connected.sort_unstable_by_key(|info| *info.last_connected());

            for _ in 0..number_to_disconnect {
                if let Some(peer_info) = connected.pop() {
                    let addr = *peer_info.address();
                    debug!("Disconnecting from {}", addr);
                    self.inbound
                        .route(Message::new(Direction::Internal, Payload::Disconnect(addr)))
                        .await;
                }
            }
        }

        if number_of_connected_peers != 0 {
            // Broadcast a `Version` request to each connected peer.
            self.broadcast_version_requests();

            // Store the peer book to storage.
            self.save_peer_book_to_storage()?;
        }

        Ok(())
    }

    ///
    /// Returns `true` if the given address is connecting with this node.
    ///
    #[inline]
    pub fn is_connecting(&self, address: &SocketAddr) -> bool {
        self.peer_book.read().is_connecting(address)
    }

    ///
    /// Returns `true` if the given address is connected with this node.
    ///
    #[inline]
    pub fn is_connected(&self, address: &SocketAddr) -> bool {
        self.peer_book.read().is_connected(address)
    }

    ///
    /// Returns `true` if the given address is a disconnected peer of this node.
    ///
    #[inline]
    pub fn is_disconnected(&self, address: &SocketAddr) -> bool {
        self.peer_book.read().is_disconnected(address)
    }

    ///
    /// Returns the number of peers connected to this node.
    ///
    #[inline]
    pub fn number_of_connected_peers(&self) -> u16 {
        self.peer_book.read().number_of_connected_peers()
    }

    ///
    /// Returns a map of all connected peers with their peer-specific information.
    ///
    #[inline]
    pub fn connected_peers(&self) -> HashMap<SocketAddr, PeerInfo> {
        self.peer_book.read().connected_peers().clone()
    }

    ///
    /// Returns the `SocketAddr` of the last seen peer to be used as a sync node, or `None`.
    ///
    pub fn last_seen(&self) -> Option<SocketAddr> {
        if let Some((&socket_address, _)) = self
            .connected_peers()
            .iter()
            .max_by(|a, b| a.1.last_seen().cmp(&b.1.last_seen()))
        {
            Some(socket_address)
        } else {
            None
        }
    }

    ///
    /// Returns a map of all disconnected peers with their peer-specific information.
    ///
    #[inline]
    pub fn disconnected_peers(&self) -> HashMap<SocketAddr, PeerInfo> {
        self.peer_book.read().disconnected_peers().clone()
    }

    ///
    /// Adds the given address to the disconnected peers in this peer book.
    ///
    #[inline]
    pub fn add_peer(&self, address: &SocketAddr) -> Result<(), NetworkError> {
        self.peer_book.write().add_peer(address)
    }

    ///
    /// Returns the local address of the node.
    ///
    #[inline]
    pub fn local_address(&self) -> Option<SocketAddr> {
        self.environment.local_address()
    }

    ///
    /// Returns the current handshake nonce for the given connected peer.
    ///
    #[inline]
    fn nonce(&self, remote_address: &SocketAddr) -> Result<u64, NetworkError> {
        self.peer_book.read().handshake_nonce(remote_address)
    }

    async fn initiate_connection(&self, remote_address: SocketAddr) -> Result<(), NetworkError> {
        let own_address = self.local_address().unwrap(); // must be known by now
        if remote_address == own_address {
            return Err(NetworkError::SelfConnectAttempt);
        }
        if self.is_connecting(&remote_address) {
            return Err(NetworkError::PeerAlreadyConnecting);
        }
        if self.is_connected(&remote_address) {
            return Err(NetworkError::PeerAlreadyConnected);
        }

        // open the connection
        let stream = TcpStream::connect(remote_address).await?;

        let (reader, writer) = stream.into_split();
        let writer = ConnWriter::new(remote_address, writer);
        let mut reader = ConnReader::new(remote_address, reader);

        let block_height = self.environment.current_block_height();
        // TODO (raychu86): Establish a formal node version.
        let version = Version::new_with_rng(1u64, block_height, own_address.port());

        // Set the peer as a connecting peer in the peer book.
        self.connecting_to_peer(remote_address, version.nonce)?;

        // Send a connection request with the outbound handler.
        writer.write_message(&Payload::Version(version.clone())).await?;

        let message = match reader.read_message().await {
            Ok(inbound_message) => inbound_message,
            Err(e) => {
                error!("An error occurred while handshaking with {}: {}", remote_address, e);
                return Err(NetworkError::InvalidHandshake);
            }
        };

        if let Payload::Verack(_) = message.payload {
            let message = match reader.read_message().await {
                Ok(inbound_message) => inbound_message,
                Err(e) => {
                    error!("An error occurred while handshaking with {}: {}", remote_address, e);
                    return Err(NetworkError::InvalidHandshake);
                }
            };

            if let Payload::Version(_) = message.payload {
                let verack = Verack::new(version.nonce);
                writer.write_message(&Payload::Verack(verack)).await?;

                // spawn the inbound loop
                let inbound = self.inbound.clone();
                tokio::spawn(async move {
                    inbound.listen_for_messages(&mut reader).await;
                });

                // save the outbound channel
                self.outbound.channels.write().insert(remote_address, writer);

                self.connected_to_peer(remote_address, version.nonce)
            } else {
                Err(NetworkError::InvalidHandshake)
            }
        } else {
            error!("{} didn't respond with a Verack during the handshake", remote_address);
            self.disconnected_from_peer(&remote_address)?;
            Err(NetworkError::InvalidHandshake)
        }
    }

    ///
    /// Broadcasts a connection request to all default bootnodes of the network.
    ///
    /// This function attempts to reconnect this node server with any bootnode peer
    /// that this node may have failed to connect to.
    ///
    /// This function filters out any bootnode peers the node server is already connected to.
    ///
    async fn connect_to_bootnodes(&self) {
        trace!("Connecting to default bootnodes");

        // Fetch the current connected peers of this node.
        let connected_peers = self.connected_peers();

        // Iterate through each bootnode address and attempt a connection request.
        for bootnode_address in self
            .environment
            .bootnodes()
            .iter()
            .filter(|addr| !connected_peers.contains_key(addr))
        {
            if let Err(e) = self.initiate_connection(*bootnode_address).await {
                warn!("Couldn't connect to bootnode {}: {}", bootnode_address, e);
            }
        }
    }

    /// Broadcasts a connection request to all disconnected peers.
    async fn connect_to_disconnected_peers(&self) {
        trace!("Connecting to disconnected peers");

        // Iterate through each connected peer and attempts a connection request.
        for (remote_address, _) in self.disconnected_peers() {
            if let Err(e) = self.initiate_connection(remote_address).await {
                warn!("Couldn't connect to the disconnected peer {}: {}", remote_address, e);
            }
        }
    }

    /// Broadcasts a `Version` message to all connected peers.
    fn broadcast_version_requests(&self) {
        // Get the local address of this node.
        let local_address = self.local_address().unwrap(); // must be known by now
        // Fetch the current block height of this node.
        let block_height = self.environment.current_block_height();

        // Broadcast a `Version` message to each connected peer of this node server.
        trace!("Broadcasting Version messages");
        for (remote_address, _) in self.connected_peers() {
            // Get the handshake nonce.
            if let Ok(nonce) = self.nonce(&remote_address) {
                // Case 1 - The remote address is of a connected peer and the nonce was retrieved.

                // TODO (raychu86): Establish a formal node version.
                // Broadcast a `Version` message to the connected peer.
                self.outbound.send_request(Message::new(
                    Direction::Outbound(remote_address),
                    Payload::Version(Version::new(1u64, block_height, nonce, local_address.port())),
                ));
            } else {
                // Case 2 - The remote address is not of a connected peer, proceed to disconnect.

                // Disconnect from the peer if there is no active connection channel
                // TODO (howardwu): Inform Outbound to also disconnect, by dropping any channels held with this peer.
                if let Err(e) = self.disconnected_from_peer(&remote_address) {
                    warn!("Couldn't mark {} as disconnected: {}", remote_address, e);
                }
            };
        }
    }

    /// Broadcasts a `GetPeers` message to all connected peers to request for more peers.
    fn broadcast_getpeers_requests(&self) {
        trace!("Sending GetPeers requests to connected peers");

        for (remote_address, _) in self.connected_peers() {
            self.outbound
                .send_request(Message::new(Direction::Outbound(remote_address), Payload::GetPeers));

            // // Fetch the connection channel.
            // if let Some(channel) = self.get_channel(&remote_address) {
            //     // Broadcast the message over the channel.
            //     if let Err(_) = channel.write(&GetPeers).await {
            //         // Disconnect from the peer if the message fails to send.
            //         self.disconnected_from_peer(&remote_address).await?;
            //     }
            // } else {
            //     // Disconnect from the peer if the channel is not active.
            //     self.disconnected_from_peer(&remote_address).await?;
            // }
        }
    }

    /// TODO (howardwu): Implement manual serializers and deserializers to prevent forward breakage
    ///  when the PeerBook or PeerInfo struct fields change.
    ///
    /// Stores the current peer book to the given storage object.
    ///
    /// This function checks that this node is not connected to itself,
    /// and proceeds to serialize the peer book into a byte vector for storage.
    ///
    #[inline]
    fn save_peer_book_to_storage(&self) -> Result<(), NetworkError> {
        // Serialize the peer book.
        let serialized_peer_book = bincode::serialize(&*self.peer_book.read())?;

        // Save the serialized peer book to storage.
        self.environment
            .storage()
            .write()
            .save_peer_book_to_storage(serialized_peer_book)?;

        Ok(())
    }
}

impl Peers {
    ///
    /// Sets the given remote address and nonce in the peer book as connecting to this node server.
    ///
    #[inline]
    pub(crate) fn connecting_to_peer(&self, remote_address: SocketAddr, nonce: u64) -> Result<(), NetworkError> {
        // Set the peer as connecting with this node server.
        self.peer_book.write().set_connecting(&remote_address, nonce)
    }

    ///
    /// Sets the given remote address in the peer book as connected to this node server.
    ///
    #[inline]
    pub(crate) fn connected_to_peer(&self, remote_address: SocketAddr, nonce: u64) -> Result<(), NetworkError> {
        self.peer_book.write().set_connected(remote_address, nonce)
    }

    /// TODO (howardwu): Add logic to remove the active channels
    ///  and handshakes of the peer from this struct.
    /// Sets the given remote address in the peer book as disconnected from this node server.
    ///
    #[inline]
    pub(crate) fn disconnected_from_peer(&self, remote_address: &SocketAddr) -> Result<(), NetworkError> {
        self.peer_book.write().set_disconnected(remote_address)
        // TODO (howardwu): Attempt to blindly send disconnect message to peer.
    }

    pub(crate) fn version_to_verack(
        &self,
        remote_address: SocketAddr,
        remote_version: &Version,
    ) -> Result<(), NetworkError> {
        // FIXME(ljedrz): it appears that Verack is not sent back in a 1:1 fashion
        if self.number_of_connected_peers() < self.environment.maximum_number_of_connected_peers() {
            self.outbound.send_request(Message::new(
                Direction::Outbound(remote_address),
                Payload::Verack(Verack::new(remote_version.nonce)),
            ));

            if !self.connected_peers().contains_key(&remote_address) {
                self.connecting_to_peer(remote_address, remote_version.nonce)?;
            }
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn verack(&self, _remote_verack: &Verack) {}

    pub(crate) fn send_get_peers(&self, remote_address: SocketAddr) {
        // TODO (howardwu): Simplify this and parallelize this with Rayon.
        // Broadcast the sanitized list of connected peers back to requesting peer.
        let mut peers = Vec::new();
        for (peer_address, peer_info) in self.connected_peers() {
            // Skip the iteration if the requesting peer that we're sending the response to
            // appears in the list of peers.
            if peer_address == remote_address {
                continue;
            }
            peers.push((peer_address, *peer_info.last_seen()));
        }
        self.outbound
            .send_request(Message::new(Direction::Outbound(remote_address), Payload::Peers(peers)));
    }

    /// A miner has sent their list of peer addresses.
    /// Add all new/updated addresses to our disconnected.
    /// The connection handler will be responsible for sending out handshake requests to them.
    pub(crate) fn process_inbound_peers(&self, peers: Vec<Peer>) -> Result<(), NetworkError> {
        // TODO (howardwu): Simplify this and parallelize this with Rayon.
        // Process all of the peers sent in the message,
        // by informing the peer book of that we found peers.
        let local_address = self.environment.local_address().unwrap(); // the address must be known by now

        let number_of_connected_peers = self.number_of_connected_peers();
        let number_to_connect = self
            .environment
            .maximum_number_of_connected_peers()
            .saturating_sub(number_of_connected_peers);

        for peer_address in peers
            .iter()
            .take(number_to_connect as usize)
            .map(|(addr, _)| addr)
            .filter(|&peer_addr| *peer_addr != local_address)
        {
            // Inform the peer book that we found a peer.
            // The peer book will determine if we have seen the peer before,
            // and include the peer if it is new.
            if !self.is_connecting(peer_address) && !self.is_connected(peer_address) {
                self.add_peer(peer_address)?;
            }
        }

        Ok(())
    }
}
