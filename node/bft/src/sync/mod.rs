// Copyright 2024 Aleo Network Foundation
// This file is part of the snarkOS library.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:

// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{
    helpers::{fmt_id, max_redundant_requests, BFTSender, Pending, Storage, SyncReceiver},
    spawn_blocking,
    Gateway,
    Transport,
    MAX_FETCH_TIMEOUT_IN_MS,
    PRIMARY_PING_IN_MS,
};
use snarkos_node_bft_events::{CertificateRequest, CertificateResponse, Event};
use snarkos_node_bft_ledger_service::LedgerService;
use snarkos_node_sync::{locators::BlockLocators, BlockSync, BlockSyncMode};
use snarkvm::{
    console::{network::Network, types::Field},
    ledger::{authority::Authority, block::Block, narwhal::BatchCertificate},
    prelude::{cfg_into_iter, cfg_iter},
};

use anyhow::{bail, Result};
use parking_lot::Mutex;
use rayon::prelude::*;
use std::{collections::HashMap, future::Future, net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    sync::{oneshot, Mutex as TMutex, OnceCell},
    task::JoinHandle,
};

#[derive(Clone)]
pub struct Sync<N: Network> {
    /// The gateway.
    gateway: Gateway<N>,
    /// The storage.
    storage: Storage<N>,
    /// The ledger service.
    ledger: Arc<dyn LedgerService<N>>,
    /// The block sync module.
    block_sync: BlockSync<N>,
    /// The pending certificates queue.
    pending: Arc<Pending<Field<N>, BatchCertificate<N>>>,
    /// The BFT sender.
    bft_sender: Arc<OnceCell<BFTSender<N>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// The response lock.
    response_lock: Arc<TMutex<()>>,
    /// The sync lock.
    sync_lock: Arc<TMutex<()>>,
    /// The latest block responses.
    latest_block_responses: Arc<TMutex<HashMap<u32, Block<N>>>>,
}

impl<N: Network> Sync<N> {
    /// Initializes a new sync instance.
    pub fn new(gateway: Gateway<N>, storage: Storage<N>, ledger: Arc<dyn LedgerService<N>>) -> Self {
        // Initialize the block sync module.
        let block_sync = BlockSync::new(BlockSyncMode::Gateway, ledger.clone());
        // Return the sync instance.
        Self {
            gateway,
            storage,
            ledger,
            block_sync,
            pending: Default::default(),
            bft_sender: Default::default(),
            handles: Default::default(),
            response_lock: Default::default(),
            sync_lock: Default::default(),
            latest_block_responses: Default::default(),
        }
    }

    /// Initializes the sync module and sync the storage with the ledger at bootup.
    pub async fn initialize(&self, bft_sender: Option<BFTSender<N>>) -> Result<()> {
        // If a BFT sender was provided, set it.
        if let Some(bft_sender) = bft_sender {
            self.bft_sender.set(bft_sender).expect("BFT sender already set in gateway");
        }

        info!("Syncing storage with the ledger...");

        // Sync the storage with the ledger.
        self.sync_storage_with_ledger_at_bootup().await
    }

    /// Starts the sync module.
    pub async fn run(&self, sync_receiver: SyncReceiver<N>) -> Result<()> {
        info!("Starting the sync module...");

        // Start the block sync loop.
        let self_ = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            // Sleep briefly to allow an initial primary ping to come in prior to entering the loop.
            // Ideally, a node does not consider itself synced when it has not received
            // any block locators from peer. However, in the initial bootup of validators,
            // this needs to happen, so we use this additional sleep as a grace period.
            tokio::time::sleep(Duration::from_millis(PRIMARY_PING_IN_MS)).await;
            loop {
                // Sleep briefly to avoid triggering spam detection.
                tokio::time::sleep(Duration::from_millis(PRIMARY_PING_IN_MS)).await;
                // Perform the sync routine.
                let communication = &self_.gateway;
                // let communication = &node.router;
                self_.block_sync.try_block_sync(communication).await;

                // Sync the storage with the blocks.
                if let Err(e) = self_.sync_storage_with_blocks().await {
                    error!("Unable to sync storage with blocks - {e}");
                }

                // If the node is synced, clear the `latest_block_responses`.
                if self_.is_synced() {
                    self_.latest_block_responses.lock().await.clear();
                }
            }
        }));

        // Start the pending queue expiration loop.
        let self_ = self.clone();
        self.spawn(async move {
            loop {
                // Sleep briefly.
                tokio::time::sleep(Duration::from_millis(MAX_FETCH_TIMEOUT_IN_MS)).await;

                // Remove the expired pending transmission requests.
                let self__ = self_.clone();
                let _ = spawn_blocking!({
                    self__.pending.clear_expired_callbacks();
                    Ok(())
                });
            }
        });

        // Retrieve the sync receiver.
        let SyncReceiver {
            mut rx_block_sync_advance_with_sync_blocks,
            mut rx_block_sync_remove_peer,
            mut rx_block_sync_update_peer_locators,
            mut rx_certificate_request,
            mut rx_certificate_response,
        } = sync_receiver;

        // Process the block sync request to advance with sync blocks.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, blocks, callback)) = rx_block_sync_advance_with_sync_blocks.recv().await {
                // Process the block response.
                if let Err(e) = self_.block_sync.process_block_response(peer_ip, blocks) {
                    // Send the error to the callback.
                    callback.send(Err(e)).ok();
                    continue;
                }

                // Sync the storage with the blocks.
                if let Err(e) = self_.sync_storage_with_blocks().await {
                    // Send the error to the callback.
                    callback.send(Err(e)).ok();
                    continue;
                }

                // Send the result to the callback.
                callback.send(Ok(())).ok();
            }
        });

        // Process the block sync request to remove the peer.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some(peer_ip) = rx_block_sync_remove_peer.recv().await {
                self_.block_sync.remove_peer(&peer_ip);
            }
        });

        // Process the block sync request to update peer locators.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, locators, callback)) = rx_block_sync_update_peer_locators.recv().await {
                let self_clone = self_.clone();
                tokio::spawn(async move {
                    // Update the peer locators.
                    let result = self_clone.block_sync.update_peer_locators(peer_ip, locators);
                    // Send the result to the callback.
                    callback.send(result).ok();
                });
            }
        });

        // Process the certificate request.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, certificate_request)) = rx_certificate_request.recv().await {
                self_.send_certificate_response(peer_ip, certificate_request);
            }
        });

        // Process the certificate response.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((peer_ip, certificate_response)) = rx_certificate_response.recv().await {
                self_.finish_certificate_request(peer_ip, certificate_response)
            }
        });

        Ok(())
    }
}

// Methods to manage storage.
impl<N: Network> Sync<N> {
    /// Syncs the storage with the ledger at bootup.
    pub async fn sync_storage_with_ledger_at_bootup(&self) -> Result<()> {
        // Retrieve the latest block in the ledger.
        let latest_block = self.ledger.latest_block();

        // Retrieve the block height.
        let block_height = latest_block.height();
        // Determine the number of maximum number of blocks that would have been garbage collected.
        let max_gc_blocks = u32::try_from(self.storage.max_gc_rounds())?.saturating_div(2);
        // Determine the earliest height, conservatively set to the block height minus the max GC rounds.
        // By virtue of the BFT protocol, we can guarantee that all GC range blocks will be loaded.
        let gc_height = block_height.saturating_sub(max_gc_blocks);
        // Retrieve the blocks.
        let blocks = self.ledger.get_blocks(gc_height..block_height.saturating_add(1))?;

        // Acquire the sync lock.
        let _lock = self.sync_lock.lock().await;

        debug!("Syncing storage with the ledger from block {} to {}...", gc_height, block_height.saturating_add(1));

        /* Sync storage */

        // Sync the height with the block.
        self.storage.sync_height_with_block(latest_block.height());
        // Sync the round with the block.
        self.storage.sync_round_with_block(latest_block.round());
        // Perform GC on the latest block round.
        self.storage.garbage_collect_certificates(latest_block.round());
        // Iterate over the blocks.
        for block in &blocks {
            // If the block authority is a subdag, then sync the batch certificates with the block.
            if let Authority::Quorum(subdag) = block.authority() {
                // Reconstruct the unconfirmed transactions.
                let unconfirmed_transactions = cfg_iter!(block.transactions())
                    .filter_map(|tx| {
                        tx.to_unconfirmed_transaction().map(|unconfirmed| (unconfirmed.id(), unconfirmed)).ok()
                    })
                    .collect::<HashMap<_, _>>();

                // Iterate over the certificates.
                for certificates in subdag.values().cloned() {
                    cfg_into_iter!(certificates).for_each(|certificate| {
                        self.storage.sync_certificate_with_block(block, certificate, &unconfirmed_transactions);
                    });
                }
            }
        }

        /* Sync the BFT DAG */

        // Construct a list of the certificates.
        let certificates = blocks
            .iter()
            .flat_map(|block| {
                match block.authority() {
                    // If the block authority is a beacon, then skip the block.
                    Authority::Beacon(_) => None,
                    // If the block authority is a subdag, then retrieve the certificates.
                    Authority::Quorum(subdag) => Some(subdag.values().flatten().cloned().collect::<Vec<_>>()),
                }
            })
            .flatten()
            .collect::<Vec<_>>();

        // If a BFT sender was provided, send the certificates to the BFT.
        if let Some(bft_sender) = self.bft_sender.get() {
            // Await the callback to continue.
            if let Err(e) = bft_sender.tx_sync_bft_dag_at_bootup.send(certificates).await {
                bail!("Failed to update the BFT DAG from sync: {e}");
            }
        }

        Ok(())
    }

    /// Syncs the storage with the given blocks.
    pub async fn sync_storage_with_blocks(&self) -> Result<()> {
        // Acquire the response lock.
        let _lock = self.response_lock.lock().await;

        // Retrieve the latest block height.
        let mut current_height = self.ledger.latest_block_height() + 1;

        // Retrieve the maximum block height of the peers.
        let tip = self.block_sync.find_sync_peers().map(|(x, _)| x.into_values().max().unwrap_or(0)).unwrap_or(0);
        // Determine the number of maximum number of blocks that would have been garbage collected.
        let max_gc_blocks = u32::try_from(self.storage.max_gc_rounds())?.saturating_div(2);
        // Determine the maximum height that the peer would have garbage collected.
        let max_gc_height = tip.saturating_sub(max_gc_blocks);

        // Determine if we can sync the ledger without updating the BFT first.
        if current_height <= max_gc_height {
            // Try to advance the ledger *to tip* without updating the BFT.
            while let Some(block) = self.block_sync.process_next_block(current_height) {
                info!("Syncing the ledger to block {}...", block.height());
                self.sync_ledger_with_block_without_bft(block).await?;
                // Update the current height.
                current_height += 1;
            }
            // Sync the storage with the ledger if we should transition to the BFT sync.
            if current_height > max_gc_height {
                if let Err(e) = self.sync_storage_with_ledger_at_bootup().await {
                    error!("BFT sync (with bootup routine) failed - {e}");
                }
            }
        }

        // Try to advance the ledger with sync blocks.
        while let Some(block) = self.block_sync.process_next_block(current_height) {
            info!("Syncing the BFT to block {}...", block.height());
            // Sync the storage with the block.
            self.sync_storage_with_block(block).await?;
            // Update the current height.
            current_height += 1;
        }
        Ok(())
    }

    /// Syncs the ledger with the given block without updating the BFT.
    async fn sync_ledger_with_block_without_bft(&self, block: Block<N>) -> Result<()> {
        // Acquire the sync lock.
        let _lock = self.sync_lock.lock().await;

        let self_ = self.clone();
        tokio::task::spawn_blocking(move || {
            // Check the next block.
            self_.ledger.check_next_block(&block)?;
            // Attempt to advance to the next block.
            self_.ledger.advance_to_next_block(&block)?;

            // Sync the height with the block.
            self_.storage.sync_height_with_block(block.height());
            // Sync the round with the block.
            self_.storage.sync_round_with_block(block.round());

            Ok(())
        })
        .await?
    }

    /// Syncs the storage with the given blocks.
    pub async fn sync_storage_with_block(&self, block: Block<N>) -> Result<()> {
        // Acquire the sync lock.
        let _lock = self.sync_lock.lock().await;
        // Acquire the latest block responses lock.
        let mut latest_block_responses = self.latest_block_responses.lock().await;

        // If this block has already been processed, return early.
        if self.ledger.contains_block_height(block.height()) || latest_block_responses.contains_key(&block.height()) {
            return Ok(());
        }

        // If the block authority is a subdag, then sync the batch certificates with the block.
        if let Authority::Quorum(subdag) = block.authority() {
            // Reconstruct the unconfirmed transactions.
            let unconfirmed_transactions = cfg_iter!(block.transactions())
                .filter_map(|tx| {
                    tx.to_unconfirmed_transaction().map(|unconfirmed| (unconfirmed.id(), unconfirmed)).ok()
                })
                .collect::<HashMap<_, _>>();

            // Iterate over the certificates.
            for certificates in subdag.values().cloned() {
                cfg_into_iter!(certificates.clone()).for_each(|certificate| {
                    // Sync the batch certificate with the block.
                    self.storage.sync_certificate_with_block(&block, certificate.clone(), &unconfirmed_transactions);
                });

                // Sync the BFT DAG with the certificates.
                for certificate in certificates {
                    // If a BFT sender was provided, send the certificate to the BFT.
                    if let Some(bft_sender) = self.bft_sender.get() {
                        // Await the callback to continue.
                        if let Err(e) = bft_sender.send_sync_bft(certificate).await {
                            bail!("Sync - {e}");
                        };
                    }
                }
            }
        }

        // Fetch the latest block height.
        let latest_block_height = self.ledger.latest_block_height();

        // Insert the latest block response.
        latest_block_responses.insert(block.height(), block);
        // Clear the latest block responses of older blocks.
        latest_block_responses.retain(|height, _| *height > latest_block_height);

        // Get a list of contiguous blocks from the latest block responses.
        let contiguous_blocks: Vec<Block<N>> = (latest_block_height.saturating_add(1)..)
            .take_while(|&k| latest_block_responses.contains_key(&k))
            .filter_map(|k| latest_block_responses.get(&k).cloned())
            .collect();

        // Check if the block response is ready to be added to the ledger.
        // Ensure that the previous block's leader certificate meets the availability threshold
        // based on the certificates in the current block.
        // If the availability threshold is not met, process the next block and check if it is linked to the current block.
        // Note: We do not advance to the most recent block response because we would be unable to
        // validate if the leader certificate in the block has been certified properly.
        for next_block in contiguous_blocks.into_iter() {
            // Retrieve the height of the next block.
            let next_block_height = next_block.height();

            // Fetch the leader certificate and the relevant rounds.
            let leader_certificate = match next_block.authority() {
                Authority::Quorum(subdag) => subdag.leader_certificate().clone(),
                _ => bail!("Received a block with an unexpected authority type."),
            };
            let commit_round = leader_certificate.round();
            let certificate_round = commit_round.saturating_add(1);

            // Get the committee lookback for the commit round.
            let committee_lookback = self.ledger.get_committee_lookback_for_round(commit_round)?;
            // Retrieve all of the certificates for the **certificate** round.
            let certificates = self.storage.get_certificates_for_round(certificate_round);
            // Construct a set over the authors who included the leader's certificate in the certificate round.
            let authors = certificates
                .iter()
                .filter_map(|c| match c.previous_certificate_ids().contains(&leader_certificate.id()) {
                    true => Some(c.author()),
                    false => None,
                })
                .collect();

            debug!("Validating sync block {next_block_height} at round {commit_round}...");
            // Check if the leader is ready to be committed.
            if committee_lookback.is_availability_threshold_reached(&authors) {
                // Initialize the current certificate.
                let mut current_certificate = leader_certificate;
                // Check if there are any linked blocks that need to be added.
                let mut blocks_to_add = vec![next_block];

                // Check if there are other blocks to process based on `is_linked`.
                for height in (self.ledger.latest_block_height().saturating_add(1)..next_block_height).rev() {
                    // Retrieve the previous block.
                    let Some(previous_block) = latest_block_responses.get(&height) else {
                        bail!("Block {height} is missing from the latest block responses.");
                    };
                    // Retrieve the previous certificate.
                    let previous_certificate = match previous_block.authority() {
                        Authority::Quorum(subdag) => subdag.leader_certificate().clone(),
                        _ => bail!("Received a block with an unexpected authority type."),
                    };
                    // Determine if there is a path between the previous certificate and the current certificate.
                    if self.is_linked(previous_certificate.clone(), current_certificate.clone())? {
                        debug!("Previous sync block {height} is linked to the current block {next_block_height}");
                        // Add the previous leader certificate to the list of certificates to commit.
                        blocks_to_add.insert(0, previous_block.clone());
                        // Update the current certificate to the previous leader certificate.
                        current_certificate = previous_certificate;
                    }
                }

                // Add the blocks to the ledger.
                for block in blocks_to_add {
                    // Check that the blocks are sequential and can be added to the ledger.
                    let block_height = block.height();
                    if block_height != self.ledger.latest_block_height().saturating_add(1) {
                        warn!("Skipping block {block_height} from the latest block responses - not sequential.");
                        continue;
                    }

                    let self_ = self.clone();
                    tokio::task::spawn_blocking(move || {
                        // Check the next block.
                        self_.ledger.check_next_block(&block)?;
                        // Attempt to advance to the next block.
                        self_.ledger.advance_to_next_block(&block)?;

                        // Sync the height with the block.
                        self_.storage.sync_height_with_block(block.height());
                        // Sync the round with the block.
                        self_.storage.sync_round_with_block(block.round());

                        Ok::<(), anyhow::Error>(())
                    })
                    .await??;
                    // Remove the block height from the latest block responses.
                    latest_block_responses.remove(&block_height);
                }
            } else {
                debug!(
                    "Availability threshold was not reached for block {next_block_height} at round {commit_round}. Checking next block..."
                );
            }
        }

        Ok(())
    }

    /// Returns `true` if there is a path from the previous certificate to the current certificate.
    fn is_linked(
        &self,
        previous_certificate: BatchCertificate<N>,
        current_certificate: BatchCertificate<N>,
    ) -> Result<bool> {
        // Initialize the list containing the traversal.
        let mut traversal = vec![current_certificate.clone()];
        // Iterate over the rounds from the current certificate to the previous certificate.
        for round in (previous_certificate.round()..current_certificate.round()).rev() {
            // Retrieve all of the certificates for this past round.
            let certificates = self.storage.get_certificates_for_round(round);
            // Filter the certificates to only include those that are in the traversal.
            traversal = certificates
                .into_iter()
                .filter(|p| traversal.iter().any(|c| c.previous_certificate_ids().contains(&p.id())))
                .collect();
        }
        Ok(traversal.contains(&previous_certificate))
    }
}

// Methods to assist with the block sync module.
impl<N: Network> Sync<N> {
    /// Returns `true` if the node is synced and has connected peers.
    pub fn is_synced(&self) -> bool {
        if self.gateway.number_of_connected_peers() == 0 {
            return false;
        }
        self.block_sync.is_block_synced()
    }

    /// Returns the number of blocks the node is behind the greatest peer height.
    pub fn num_blocks_behind(&self) -> u32 {
        self.block_sync.num_blocks_behind()
    }

    /// Returns `true` if the node is in gateway mode.
    pub const fn is_gateway_mode(&self) -> bool {
        self.block_sync.mode().is_gateway()
    }

    /// Returns the current block locators of the node.
    pub fn get_block_locators(&self) -> Result<BlockLocators<N>> {
        self.block_sync.get_block_locators()
    }

    /// Returns the block sync module.
    #[cfg(test)]
    #[doc(hidden)]
    pub(super) fn block_sync(&self) -> &BlockSync<N> {
        &self.block_sync
    }
}

// Methods to assist with fetching batch certificates from peers.
impl<N: Network> Sync<N> {
    /// Sends a certificate request to the specified peer.
    pub async fn send_certificate_request(
        &self,
        peer_ip: SocketAddr,
        certificate_id: Field<N>,
    ) -> Result<BatchCertificate<N>> {
        // Initialize a oneshot channel.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Determine how many sent requests are pending.
        let num_sent_requests = self.pending.num_sent_requests(certificate_id);
        // Determine if we've already sent a request to the peer.
        let contains_peer_with_sent_request = self.pending.contains_peer_with_sent_request(certificate_id, peer_ip);
        // Determine the maximum number of redundant requests.
        let num_redundant_requests = max_redundant_requests(self.ledger.clone(), self.storage.current_round());
        // Determine if we should send a certificate request to the peer.
        // We send at most `num_redundant_requests` requests and each peer can only receive one request at a time.
        let should_send_request = num_sent_requests < num_redundant_requests && !contains_peer_with_sent_request;

        // Insert the certificate ID into the pending queue.
        self.pending.insert(certificate_id, peer_ip, Some((callback_sender, should_send_request)));

        // If the number of requests is less than or equal to the redundancy factor, send the certificate request to the peer.
        if should_send_request {
            // Send the certificate request to the peer.
            if self.gateway.send(peer_ip, Event::CertificateRequest(certificate_id.into())).await.is_none() {
                bail!("Unable to fetch batch certificate {certificate_id} - failed to send request")
            }
        } else {
            debug!(
                "Skipped sending request for certificate {} to '{peer_ip}' ({num_sent_requests} redundant requests)",
                fmt_id(certificate_id)
            );
        }
        // Wait for the certificate to be fetched.
        match tokio::time::timeout(Duration::from_millis(MAX_FETCH_TIMEOUT_IN_MS), callback_receiver).await {
            // If the certificate was fetched, return it.
            Ok(result) => Ok(result?),
            // If the certificate was not fetched, return an error.
            Err(e) => bail!("Unable to fetch certificate {} - (timeout) {e}", fmt_id(certificate_id)),
        }
    }

    /// Handles the incoming certificate request.
    fn send_certificate_response(&self, peer_ip: SocketAddr, request: CertificateRequest<N>) {
        // Attempt to retrieve the certificate.
        if let Some(certificate) = self.storage.get_certificate(request.certificate_id) {
            // Send the certificate response to the peer.
            let self_ = self.clone();
            tokio::spawn(async move {
                let _ = self_.gateway.send(peer_ip, Event::CertificateResponse(certificate.into())).await;
            });
        }
    }

    /// Handles the incoming certificate response.
    /// This method ensures the certificate response is well-formed and matches the certificate ID.
    fn finish_certificate_request(&self, peer_ip: SocketAddr, response: CertificateResponse<N>) {
        let certificate = response.certificate;
        // Check if the peer IP exists in the pending queue for the given certificate ID.
        let exists = self.pending.get_peers(certificate.id()).unwrap_or_default().contains(&peer_ip);
        // If the peer IP exists, finish the pending request.
        if exists {
            // TODO: Validate the certificate.
            // Remove the certificate ID from the pending queue.
            self.pending.remove(certificate.id(), Some(certificate));
        }
    }
}

impl<N: Network> Sync<N> {
    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the primary.
    pub async fn shut_down(&self) {
        info!("Shutting down the sync module...");
        // Acquire the response lock.
        let _lock = self.response_lock.lock().await;
        // Acquire the sync lock.
        let _lock = self.sync_lock.lock().await;
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    use crate::{helpers::now, ledger_service::CoreLedgerService, storage_service::BFTMemoryService};
    use snarkos_account::Account;
    use snarkvm::{
        console::{
            account::{Address, PrivateKey},
            network::MainnetV0,
        },
        ledger::{
            narwhal::{BatchCertificate, BatchHeader, Subdag},
            store::{helpers::memory::ConsensusMemory, ConsensusStore},
        },
        prelude::{Ledger, VM},
        utilities::TestRng,
    };

    use aleo_std::StorageMode;
    use indexmap::IndexSet;
    use rand::Rng;
    use std::collections::BTreeMap;

    type CurrentNetwork = MainnetV0;
    type CurrentLedger = Ledger<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;
    type CurrentConsensusStore = ConsensusStore<CurrentNetwork, ConsensusMemory<CurrentNetwork>>;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_commit_via_is_linked() -> anyhow::Result<()> {
        let rng = &mut TestRng::default();
        // Initialize the round parameters.
        let max_gc_rounds = BatchHeader::<CurrentNetwork>::MAX_GC_ROUNDS as u64;
        let commit_round = 2;

        // Initialize the store.
        let store = CurrentConsensusStore::open(None).unwrap();
        let account: Account<CurrentNetwork> = Account::new(rng)?;

        // Create a genesis block with a seeded RNG to reproduce the same genesis private keys.
        let seed: u64 = rng.gen();
        let genesis_rng = &mut TestRng::from_seed(seed);
        let genesis = VM::from(store).unwrap().genesis_beacon(account.private_key(), genesis_rng).unwrap();

        // Extract the private keys from the genesis committee by using the same RNG to sample private keys.
        let genesis_rng = &mut TestRng::from_seed(seed);
        let private_keys = [
            *account.private_key(),
            PrivateKey::new(genesis_rng)?,
            PrivateKey::new(genesis_rng)?,
            PrivateKey::new(genesis_rng)?,
        ];

        // Initialize the ledger with the genesis block.
        let ledger = CurrentLedger::load(genesis.clone(), StorageMode::Production).unwrap();
        // Initialize the ledger.
        let core_ledger = Arc::new(CoreLedgerService::new(ledger.clone(), Default::default()));

        // Sample 5 rounds of batch certificates starting at the genesis round from a static set of 4 authors.
        let (round_to_certificates_map, committee) = {
            let addresses = vec![
                Address::try_from(private_keys[0])?,
                Address::try_from(private_keys[1])?,
                Address::try_from(private_keys[2])?,
                Address::try_from(private_keys[3])?,
            ];

            let committee = ledger.latest_committee().unwrap();

            // Initialize a mapping from the round number to the set of batch certificates in the round.
            let mut round_to_certificates_map: HashMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> =
                HashMap::new();
            let mut previous_certificates: IndexSet<BatchCertificate<CurrentNetwork>> = IndexSet::with_capacity(4);

            for round in 0..=commit_round + 8 {
                let mut current_certificates = IndexSet::new();
                let previous_certificate_ids: IndexSet<_> = if round == 0 || round == 1 {
                    IndexSet::new()
                } else {
                    previous_certificates.iter().map(|c| c.id()).collect()
                };
                let committee_id = committee.id();

                // Create a certificate for the leader.
                if round <= 5 {
                    let leader = committee.get_leader(round).unwrap();
                    let leader_index = addresses.iter().position(|&address| address == leader).unwrap();
                    let non_leader_index = addresses.iter().position(|&address| address != leader).unwrap();
                    for i in [leader_index, non_leader_index].into_iter() {
                        let batch_header = BatchHeader::new(
                            &private_keys[i],
                            round,
                            now(),
                            committee_id,
                            Default::default(),
                            previous_certificate_ids.clone(),
                            rng,
                        )
                        .unwrap();
                        // Sign the batch header.
                        let mut signatures = IndexSet::with_capacity(4);
                        for (j, private_key_2) in private_keys.iter().enumerate() {
                            if i != j {
                                signatures.insert(private_key_2.sign(&[batch_header.batch_id()], rng).unwrap());
                            }
                        }
                        current_certificates.insert(BatchCertificate::from(batch_header, signatures).unwrap());
                    }
                }

                // Create a certificate for each validator.
                if round > 5 {
                    for (i, private_key_1) in private_keys.iter().enumerate() {
                        let batch_header = BatchHeader::new(
                            private_key_1,
                            round,
                            now(),
                            committee_id,
                            Default::default(),
                            previous_certificate_ids.clone(),
                            rng,
                        )
                        .unwrap();
                        // Sign the batch header.
                        let mut signatures = IndexSet::with_capacity(4);
                        for (j, private_key_2) in private_keys.iter().enumerate() {
                            if i != j {
                                signatures.insert(private_key_2.sign(&[batch_header.batch_id()], rng).unwrap());
                            }
                        }
                        current_certificates.insert(BatchCertificate::from(batch_header, signatures).unwrap());
                    }
                }
                // Update the map of certificates.
                round_to_certificates_map.insert(round, current_certificates.clone());
                previous_certificates = current_certificates.clone();
            }
            (round_to_certificates_map, committee)
        };

        // Initialize the storage.
        let storage = Storage::new(core_ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);
        // Insert certificates into storage.
        let mut certificates: Vec<BatchCertificate<CurrentNetwork>> = Vec::new();
        for i in 1..=commit_round + 8 {
            let c = (*round_to_certificates_map.get(&i).unwrap()).clone();
            certificates.extend(c);
        }
        for certificate in certificates.clone().iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }

        // Create block 1.
        let leader_round_1 = commit_round;
        let leader_1 = committee.get_leader(leader_round_1).unwrap();
        let leader_certificate = storage.get_certificate_for_round_with_author(commit_round, leader_1).unwrap();
        let block_1 = {
            let mut subdag_map: BTreeMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> = BTreeMap::new();
            let mut leader_cert_map = IndexSet::new();
            leader_cert_map.insert(leader_certificate.clone());
            let mut previous_cert_map = IndexSet::new();
            for cert in storage.get_certificates_for_round(commit_round - 1) {
                previous_cert_map.insert(cert);
            }
            subdag_map.insert(commit_round, leader_cert_map.clone());
            subdag_map.insert(commit_round - 1, previous_cert_map.clone());
            let subdag = Subdag::from(subdag_map.clone())?;
            core_ledger.prepare_advance_to_next_quorum_block(subdag, Default::default())?
        };
        // Insert block 1.
        core_ledger.advance_to_next_block(&block_1)?;

        // Create block 2.
        let leader_round_2 = commit_round + 2;
        let leader_2 = committee.get_leader(leader_round_2).unwrap();
        let leader_certificate_2 = storage.get_certificate_for_round_with_author(leader_round_2, leader_2).unwrap();
        let block_2 = {
            let mut subdag_map_2: BTreeMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> = BTreeMap::new();
            let mut leader_cert_map_2 = IndexSet::new();
            leader_cert_map_2.insert(leader_certificate_2.clone());
            let mut previous_cert_map_2 = IndexSet::new();
            for cert in storage.get_certificates_for_round(leader_round_2 - 1) {
                previous_cert_map_2.insert(cert);
            }
            let mut prev_commit_cert_map_2 = IndexSet::new();
            for cert in storage.get_certificates_for_round(leader_round_2 - 2) {
                if cert != leader_certificate {
                    prev_commit_cert_map_2.insert(cert);
                }
            }
            subdag_map_2.insert(leader_round_2, leader_cert_map_2.clone());
            subdag_map_2.insert(leader_round_2 - 1, previous_cert_map_2.clone());
            subdag_map_2.insert(leader_round_2 - 2, prev_commit_cert_map_2.clone());
            let subdag_2 = Subdag::from(subdag_map_2.clone())?;
            core_ledger.prepare_advance_to_next_quorum_block(subdag_2, Default::default())?
        };
        // Insert block 2.
        core_ledger.advance_to_next_block(&block_2)?;

        // Create block 3
        let leader_round_3 = commit_round + 4;
        let leader_3 = committee.get_leader(leader_round_3).unwrap();
        let leader_certificate_3 = storage.get_certificate_for_round_with_author(leader_round_3, leader_3).unwrap();
        let block_3 = {
            let mut subdag_map_3: BTreeMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> = BTreeMap::new();
            let mut leader_cert_map_3 = IndexSet::new();
            leader_cert_map_3.insert(leader_certificate_3.clone());
            let mut previous_cert_map_3 = IndexSet::new();
            for cert in storage.get_certificates_for_round(leader_round_3 - 1) {
                previous_cert_map_3.insert(cert);
            }
            let mut prev_commit_cert_map_3 = IndexSet::new();
            for cert in storage.get_certificates_for_round(leader_round_3 - 2) {
                if cert != leader_certificate_2 {
                    prev_commit_cert_map_3.insert(cert);
                }
            }
            subdag_map_3.insert(leader_round_3, leader_cert_map_3.clone());
            subdag_map_3.insert(leader_round_3 - 1, previous_cert_map_3.clone());
            subdag_map_3.insert(leader_round_3 - 2, prev_commit_cert_map_3.clone());
            let subdag_3 = Subdag::from(subdag_map_3.clone())?;
            core_ledger.prepare_advance_to_next_quorum_block(subdag_3, Default::default())?
        };
        // Insert block 3.
        core_ledger.advance_to_next_block(&block_3)?;

        // Initialize the syncing ledger.
        let syncing_ledger = Arc::new(CoreLedgerService::new(
            CurrentLedger::load(genesis, StorageMode::Production).unwrap(),
            Default::default(),
        ));
        // Initialize the gateway.
        let gateway = Gateway::new(account.clone(), storage.clone(), syncing_ledger.clone(), None, &[], None)?;
        // Initialize the sync module.
        let sync = Sync::new(gateway.clone(), storage.clone(), syncing_ledger.clone());
        // Try to sync block 1.
        sync.sync_storage_with_block(block_1).await?;
        assert_eq!(syncing_ledger.latest_block_height(), 1);
        // Try to sync block 2.
        sync.sync_storage_with_block(block_2).await?;
        assert_eq!(syncing_ledger.latest_block_height(), 2);
        // Try to sync block 3.
        sync.sync_storage_with_block(block_3).await?;
        assert_eq!(syncing_ledger.latest_block_height(), 3);
        // Ensure blocks 1 and 2 were added to the ledger.
        assert!(syncing_ledger.contains_block_height(1));
        assert!(syncing_ledger.contains_block_height(2));

        Ok(())
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_pending_certificates() -> anyhow::Result<()> {
        let rng = &mut TestRng::default();
        // Initialize the round parameters.
        let max_gc_rounds = BatchHeader::<CurrentNetwork>::MAX_GC_ROUNDS as u64;
        let commit_round = 2;

        // Initialize the store.
        let store = CurrentConsensusStore::open(None).unwrap();
        let account: Account<CurrentNetwork> = Account::new(rng)?;

        // Create a genesis block with a seeded RNG to reproduce the same genesis private keys.
        let seed: u64 = rng.gen();
        let genesis_rng = &mut TestRng::from_seed(seed);
        let genesis = VM::from(store).unwrap().genesis_beacon(account.private_key(), genesis_rng).unwrap();

        // Extract the private keys from the genesis committee by using the same RNG to sample private keys.
        let genesis_rng = &mut TestRng::from_seed(seed);
        let private_keys = [
            *account.private_key(),
            PrivateKey::new(genesis_rng)?,
            PrivateKey::new(genesis_rng)?,
            PrivateKey::new(genesis_rng)?,
        ];
        // Initialize the ledger with the genesis block.
        let ledger = CurrentLedger::load(genesis.clone(), StorageMode::Production).unwrap();
        // Initialize the ledger.
        let core_ledger = Arc::new(CoreLedgerService::new(ledger.clone(), Default::default()));
        // Sample rounds of batch certificates starting at the genesis round from a static set of 4 authors.
        let (round_to_certificates_map, committee) = {
            // Initialize the committee.
            let committee = ledger.latest_committee().unwrap();
            // Initialize a mapping from the round number to the set of batch certificates in the round.
            let mut round_to_certificates_map: HashMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> =
                HashMap::new();
            let mut previous_certificates: IndexSet<BatchCertificate<CurrentNetwork>> = IndexSet::with_capacity(4);

            for round in 0..=commit_round + 8 {
                let mut current_certificates = IndexSet::new();
                let previous_certificate_ids: IndexSet<_> = if round == 0 || round == 1 {
                    IndexSet::new()
                } else {
                    previous_certificates.iter().map(|c| c.id()).collect()
                };
                let committee_id = committee.id();
                // Create a certificate for each validator.
                for (i, private_key_1) in private_keys.iter().enumerate() {
                    let batch_header = BatchHeader::new(
                        private_key_1,
                        round,
                        now(),
                        committee_id,
                        Default::default(),
                        previous_certificate_ids.clone(),
                        rng,
                    )
                    .unwrap();
                    // Sign the batch header.
                    let mut signatures = IndexSet::with_capacity(4);
                    for (j, private_key_2) in private_keys.iter().enumerate() {
                        if i != j {
                            signatures.insert(private_key_2.sign(&[batch_header.batch_id()], rng).unwrap());
                        }
                    }
                    current_certificates.insert(BatchCertificate::from(batch_header, signatures).unwrap());
                }

                // Update the map of certificates.
                round_to_certificates_map.insert(round, current_certificates.clone());
                previous_certificates = current_certificates.clone();
            }
            (round_to_certificates_map, committee)
        };

        // Initialize the storage.
        let storage = Storage::new(core_ledger.clone(), Arc::new(BFTMemoryService::new()), max_gc_rounds);
        // Insert certificates into storage.
        let mut certificates: Vec<BatchCertificate<CurrentNetwork>> = Vec::new();
        for i in 1..=commit_round + 8 {
            let c = (*round_to_certificates_map.get(&i).unwrap()).clone();
            certificates.extend(c);
        }
        for certificate in certificates.clone().iter() {
            storage.testing_only_insert_certificate_testing_only(certificate.clone());
        }
        // Create block 1.
        let leader_round_1 = commit_round;
        let leader_1 = committee.get_leader(leader_round_1).unwrap();
        let leader_certificate = storage.get_certificate_for_round_with_author(commit_round, leader_1).unwrap();
        let mut subdag_map: BTreeMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> = BTreeMap::new();
        let block_1 = {
            let mut leader_cert_map = IndexSet::new();
            leader_cert_map.insert(leader_certificate.clone());
            let mut previous_cert_map = IndexSet::new();
            for cert in storage.get_certificates_for_round(commit_round - 1) {
                previous_cert_map.insert(cert);
            }
            subdag_map.insert(commit_round, leader_cert_map.clone());
            subdag_map.insert(commit_round - 1, previous_cert_map.clone());
            let subdag = Subdag::from(subdag_map.clone())?;
            core_ledger.prepare_advance_to_next_quorum_block(subdag, Default::default())?
        };
        // Insert block 1.
        core_ledger.advance_to_next_block(&block_1)?;

        // Create block 2.
        let leader_round_2 = commit_round + 2;
        let leader_2 = committee.get_leader(leader_round_2).unwrap();
        let leader_certificate_2 = storage.get_certificate_for_round_with_author(leader_round_2, leader_2).unwrap();
        let mut subdag_map_2: BTreeMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> = BTreeMap::new();
        let block_2 = {
            let mut leader_cert_map_2 = IndexSet::new();
            leader_cert_map_2.insert(leader_certificate_2.clone());
            let mut previous_cert_map_2 = IndexSet::new();
            for cert in storage.get_certificates_for_round(leader_round_2 - 1) {
                previous_cert_map_2.insert(cert);
            }
            subdag_map_2.insert(leader_round_2, leader_cert_map_2.clone());
            subdag_map_2.insert(leader_round_2 - 1, previous_cert_map_2.clone());
            let subdag_2 = Subdag::from(subdag_map_2.clone())?;
            core_ledger.prepare_advance_to_next_quorum_block(subdag_2, Default::default())?
        };
        // Insert block 2.
        core_ledger.advance_to_next_block(&block_2)?;

        // Create block 3
        let leader_round_3 = commit_round + 4;
        let leader_3 = committee.get_leader(leader_round_3).unwrap();
        let leader_certificate_3 = storage.get_certificate_for_round_with_author(leader_round_3, leader_3).unwrap();
        let mut subdag_map_3: BTreeMap<u64, IndexSet<BatchCertificate<CurrentNetwork>>> = BTreeMap::new();
        let block_3 = {
            let mut leader_cert_map_3 = IndexSet::new();
            leader_cert_map_3.insert(leader_certificate_3.clone());
            let mut previous_cert_map_3 = IndexSet::new();
            for cert in storage.get_certificates_for_round(leader_round_3 - 1) {
                previous_cert_map_3.insert(cert);
            }
            subdag_map_3.insert(leader_round_3, leader_cert_map_3.clone());
            subdag_map_3.insert(leader_round_3 - 1, previous_cert_map_3.clone());
            let subdag_3 = Subdag::from(subdag_map_3.clone())?;
            core_ledger.prepare_advance_to_next_quorum_block(subdag_3, Default::default())?
        };
        // Insert block 3.
        core_ledger.advance_to_next_block(&block_3)?;

        /*
            Check that the pending certificates are computed correctly.
        */

        // Retrieve the pending certificates.
        let pending_certificates = storage.get_pending_certificates();
        // Check that all of the pending certificates are not contained in the ledger.
        for certificate in pending_certificates.clone() {
            assert!(!core_ledger.contains_certificate(&certificate.id()).unwrap_or(false));
        }
        // Initialize an empty set to be populated with the committed certificates in the block subdags.
        let mut committed_certificates: IndexSet<BatchCertificate<CurrentNetwork>> = IndexSet::new();
        {
            let subdag_maps = [&subdag_map, &subdag_map_2, &subdag_map_3];
            for subdag in subdag_maps.iter() {
                for subdag_certificates in subdag.values() {
                    committed_certificates.extend(subdag_certificates.iter().cloned());
                }
            }
        };
        // Create the set of candidate pending certificates as the set of all certificates minus the set of the committed certificates.
        let mut candidate_pending_certificates: IndexSet<BatchCertificate<CurrentNetwork>> = IndexSet::new();
        for certificate in certificates.clone() {
            if !committed_certificates.contains(&certificate) {
                candidate_pending_certificates.insert(certificate);
            }
        }
        // Check that the set of pending certificates is equal to the set of candidate pending certificates.
        assert_eq!(pending_certificates, candidate_pending_certificates);
        Ok(())
    }
}
