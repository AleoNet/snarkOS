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

use crate::{external::message::*, peers::PeerInfo, Environment, NetworkError, Outbound};
use snarkvm_objects::{Block, BlockHeaderHash};

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

/// A stateful component for managing the blocks for the ledger on this node server.
#[derive(Clone)]
pub struct Blocks {
    /// The parameters and settings of this node server.
    pub(crate) environment: Environment,
    /// The outbound handler of this node server.
    outbound: Arc<Outbound>,
}

impl Blocks {
    ///
    /// Creates a new instance of `Blocks`.
    ///
    pub fn new(environment: Environment, outbound: Arc<Outbound>) -> Self {
        trace!("Instantiating the block service");
        Self { environment, outbound }
    }

    ///
    /// Broadcasts updates with connected peers and maintains a permitted number of connected peers.
    ///
    pub async fn update(&self, sync_node: Option<SocketAddr>) -> Result<(), NetworkError> {
        // Check that this node is not a bootnode.
        if !self.environment.is_bootnode() {
            let block_locator_hashes = self.environment.storage().read().get_block_locator_hashes();

            if let (Some(sync_node), Ok(block_locator_hashes)) = (sync_node, block_locator_hashes) {
                // Send a GetSync to the selected sync node.
                self.outbound.send_request(Message::new(
                    Direction::Outbound(sync_node),
                    Payload::GetSync(block_locator_hashes),
                ));
            } else {
                // If no sync node is available, wait until peers have been established.
                info!("No sync node is registered, blocks could not be synced");
            }
        }

        Ok(())
    }

    ///
    /// Returns the local address of this node.
    ///
    #[inline]
    pub fn local_address(&self) -> SocketAddr {
        self.environment.local_address().unwrap() // the address must be known by now
    }

    /// Broadcast block to connected peers
    pub async fn propagate_block(
        &self,
        block_bytes: Vec<u8>,
        block_miner: SocketAddr,
        connected_peers: &HashMap<SocketAddr, PeerInfo>,
    ) -> Result<(), NetworkError> {
        debug!("Propagating a block to peers");

        let local_address = self.local_address();
        for remote_address in connected_peers.keys() {
            if *remote_address != block_miner && *remote_address != local_address {
                // Send a `Block` message to the connected peer.
                self.outbound.send_request(Message::new(
                    Direction::Outbound(*remote_address),
                    Payload::Block(block_bytes.clone()),
                ));
            }
        }

        Ok(())
    }

    /// A peer has sent us a new block to process.
    pub(crate) async fn received_block(
        &self,
        remote_address: SocketAddr,
        block: Vec<u8>,
        connected_peers: Option<HashMap<SocketAddr, PeerInfo>>,
    ) -> Result<(), NetworkError> {
        let block_struct = Block::deserialize(&block)?;
        info!(
            "Received block from epoch {} with hash {:?}",
            block_struct.header.time,
            hex::encode(block_struct.header.get_hash().0)
        );

        // Verify the block and insert it into the storage.
        if !self
            .environment
            .storage()
            .read()
            .block_hash_exists(&block_struct.header.get_hash())
        {
            let is_new_block = self
                .environment
                .consensus_parameters()
                .receive_block(
                    self.environment.dpc_parameters(),
                    &self.environment.storage().read(),
                    &mut self.environment.memory_pool().lock(),
                    &block_struct,
                )
                .is_ok();

            // This is a new block, send it to our peers.
            if let Some(connected_peers) = connected_peers {
                if is_new_block {
                    self.propagate_block(block, remote_address, &connected_peers).await?;
                }
            }
        }

        Ok(())
    }

    /// A peer has requested a block.
    pub(crate) async fn received_get_block(
        &self,
        remote_address: SocketAddr,
        header_hash: BlockHeaderHash,
    ) -> Result<(), NetworkError> {
        let block = self.environment.storage().read().get_block(&header_hash);

        if let Ok(block) = block {
            // Send a `SyncBlock` message to the connected peer.
            self.outbound.send_request(Message::new(
                Direction::Outbound(remote_address),
                Payload::SyncBlock(block.serialize()?),
            ));
        }
        Ok(())
    }

    /// A peer has requested our chain state to sync with.
    pub(crate) async fn received_get_sync(
        &self,
        remote_address: SocketAddr,
        block_locator_hashes: Vec<BlockHeaderHash>,
    ) -> Result<(), NetworkError> {
        let sync = {
            let storage_lock = self.environment.storage().read();

            let latest_shared_hash = storage_lock.get_latest_shared_hash(block_locator_hashes)?;
            let current_height = self.environment.storage().read().get_current_block_height();

            if let Ok(height) = storage_lock.get_block_number(&latest_shared_hash) {
                if height < current_height {
                    let mut max_height = current_height;

                    // if the requester is behind more than 4000 blocks
                    if height + 4000 < current_height {
                        // send the max 4000 blocks
                        max_height = height + 4000;
                    }

                    let mut block_hashes: Vec<BlockHeaderHash> = vec![];

                    for block_num in height + 1..=max_height {
                        block_hashes.push(storage_lock.get_block_hash(block_num)?);
                    }

                    // send block hashes to requester
                    block_hashes
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };

        // send a `Sync` message to the connected peer.
        self.outbound
            .send_request(Message::new(Direction::Outbound(remote_address), Payload::Sync(sync)));

        Ok(())
    }

    /// A peer has sent us their chain state.
    pub(crate) async fn received_sync(
        &self,
        remote_address: SocketAddr,
        block_hashes: Vec<BlockHeaderHash>,
    ) -> Result<(), NetworkError> {
        // If empty sync is no-op as chain states match
        if !block_hashes.is_empty() {
            // GetBlocks for each block hash: fire and forget, relying on block locator hashes to
            // detect missing blocks and divergence in chain for now.
            for hash in block_hashes {
                self.outbound.send_request(Message::new(
                    Direction::Outbound(remote_address),
                    Payload::GetBlock(hash),
                ));
            }
        }

        Ok(())
    }
}
