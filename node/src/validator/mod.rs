// Copyright (C) 2019-2023 Aleo Systems Inc.
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

mod router;

use crate::traits::NodeInterface;
use anyhow::Result;
use narwhal_config::{Committee, Import};
use narwhal_crypto::{KeyPair as NarwhalKeyPair, PublicKey};
use narwhal_node::keypair_file::read_authority_keypair_from_file;
use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
use rand::thread_rng;
use snarkos_account::Account;
use snarkos_node_bft_consensus::{
    setup::{workspace_dir, CommitteeSetup, PrimarySetup},
    BftExecutionState,
    InertConsensusInstance,
    RunningConsensusInstance,
    TransactionValidator,
};
use snarkos_node_consensus::Consensus;
use snarkos_node_messages::{BlockRequest, Message, NodeType, PuzzleResponse, UnconfirmedSolution};
use snarkos_node_rest::Rest;
use snarkos_node_router::{Heartbeat, Inbound, Outbound, Router, Routing};
use snarkos_node_tcp::{
    protocols::{Disconnect, Handshake, OnConnect, Reading, Writing},
    P2P,
};
use snarkvm::prelude::{Block, ConsensusStorage, Header, Ledger, Network, ProverSolution};
use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::task::JoinHandle;

/// A validator is a full node, capable of validating blocks.
#[derive(Clone)]
pub struct Validator<N: Network, C: ConsensusStorage<N>> {
    /// The ledger of the node.
    ledger: Ledger<N, C>,
    /// The consensus module of the node.
    consensus: Consensus<N, C>,
    /// The router of the node.
    router: Router<N>,
    /// The REST server of the node.
    rest: Option<Rest<N, C, Self>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
    /// The shutdown signal.
    shutdown: Arc<AtomicBool>,
    /// The primary keypair of the node exposed here for handshaking purposes.
    primary_keypair: Arc<NarwhalKeyPair>,
    /// Current consensus committee, might need to be mutable for dynamic committees.
    committee: Committee,
    /// The set of connected committee members by public key, no mapping is required currently but
    /// it will likely be necessary when handling dynamic committees).
    connected_committee_members: Arc<RwLock<HashMap<SocketAddr, PublicKey>>>,
    /// The running BFT consensus instance.
    bft: Arc<OnceCell<RunningConsensusInstance<BftExecutionState<N, C>>>>,
    /// A flag indicating that the BFT initialization has already begun.
    bft_started: Arc<AtomicBool>,

    dev: Option<u16>,

    processed_block: Arc<RwLock<Option<Block<N>>>>,
}

impl<N: Network, C: ConsensusStorage<N>> Validator<N, C> {
    #[allow(clippy::too_many_arguments)]
    /// Initializes a new validator node.
    pub async fn new(
        node_ip: SocketAddr,
        rest_ip: Option<SocketAddr>,
        account: Account<N>,
        trusted_peers: &[SocketAddr],
        genesis: Block<N>,
        cdn: Option<String>,
        dev: Option<u16>,
        enable_metrics: bool,
    ) -> Result<Self> {
        // Initialize the ledger.
        let ledger = Ledger::load(genesis, dev)?;
        // Initialize the CDN.
        if let Some(base_url) = cdn {
            // Sync the ledger with the CDN.
            if let Err((_, error)) = snarkos_node_cdn::sync_ledger_with_cdn(&base_url, ledger.clone()).await {
                crate::helpers::log_clean_error(dev);
                return Err(error);
            }
        }
        // Initialize the consensus.
        let consensus = Consensus::new(ledger.clone(), dev.is_some())?;

        ledger.insert_committee_member(account.clone().address());

        // Initialize the node router.
        let router = Router::new(
            node_ip,
            NodeType::Validator,
            account,
            trusted_peers,
            Self::MAXIMUM_NUMBER_OF_PEERS as u16,
            dev.is_some(),
        )
        .await?;

        // Process the committee file.
        let (primary_keypair, committee) = Self::read_committee(dev);

        // Initialize the node.
        let mut node = Self {
            ledger: ledger.clone(),
            consensus: consensus.clone(),
            router: router.clone(),
            rest: None,
            handles: Default::default(),
            shutdown: Default::default(),
            primary_keypair: primary_keypair.into(),
            committee,
            connected_committee_members: Default::default(),
            // Note: starting the BFT is called from the handshake logic once quorum is reached.
            bft: Default::default(),
            bft_started: Default::default(),
            dev,
            processed_block: Default::default(),
        };

        // Initialize the REST server.
        if let Some(rest_ip) = rest_ip {
            node.rest = Some(Rest::start(rest_ip, Some(consensus), ledger, Arc::new(node.clone()))?);
        }
        // Initialize the sync pool.
        node.initialize_sync()?;
        // Initialize the routing.
        node.initialize_routing().await;
        // Initialize the signal handler.
        node.handle_signals();

        // Initialize metrics.
        if enable_metrics {
            info!("Running with metrics enabled.");
            snarkos_node_metrics::initialize();
        }

        // Return the node.
        Ok(node)
    }

    // Reads the committee configuration and the primary's authority keypair. This is needed to
    // establish quorum before the BFT process is started.
    fn read_committee(dev: Option<u16>) -> (NarwhalKeyPair, Committee) {
        // Prepare the path containing BFT consensus files.
        let bft_path =
            format!("{}/node/bft-consensus/committee/{}", workspace_dir(), if dev.is_some() { ".dev" } else { "" });

        // In dev mode, auto-generate a permanent BFT consensus config.
        if dev.is_some() && fs::metadata(&bft_path).is_err() {
            // Prepare a source of randomness for key generation.
            let mut rng = thread_rng();

            // Hardcode the dev number of primaries, at least for now.
            const NUM_PRIMARIES: usize = 4;

            // Generate the committee setup.
            let mut primaries = Vec::with_capacity(NUM_PRIMARIES);
            for _ in 0..NUM_PRIMARIES {
                // TODO: set up a meaningful stake
                let primary = PrimarySetup::new(None, 1, vec![], &mut rng);
                primaries.push(primary);
            }
            let committee = CommitteeSetup::new(primaries, 0);

            // Create the dev subpath and write the commitee files.
            committee.write_files(true);

            // Copy the existing parameters.
            fs::copy(format!("{bft_path}/../.parameters.json"), format!("{bft_path}/.parameters.json")).unwrap();
        }

        let base_path = format!("{bft_path}{}", if dev.is_some() { "/" } else { "" });
        // If we're running dev mode, potentially use a different primary ID than 0.
        let primary_id = if let Some(dev_id) = dev { dev_id } else { 0 };

        // Load the primary's keys.
        let primary_key_file = format!("{base_path}.primary-{primary_id}-key.json");
        let primary_keypair = read_authority_keypair_from_file(primary_key_file).unwrap_or_else(|error| {
            panic!("Failed to load the node's primary keypair {base_path}.primary-{primary_id}-key.json: {error:?}")
        });

        // Read the shared files describing the committee, workers and parameters.
        let committee_file = format!("{base_path}.committee.json");
        let mut committee = Committee::import(&committee_file).expect("Failed to load the committee information");
        committee.load();

        (primary_keypair, committee)
    }

    /// Starts and sets the `RunningConsensusInstance`.
    pub async fn start_bft(&self, initial_last_executed_sub_dag_index: u64) -> Result<()> {
        // Return early if this method had already been called.
        if self.bft_started.swap(true, Ordering::Relaxed) {
            return Ok(());
        }

        let dev = self.dev;

        // Prepare the path containing BFT consensus files.
        let bft_path =
            format!("{}/node/bft-consensus/committee/{}", workspace_dir(), if dev.is_some() { ".dev" } else { "" });

        // Load the primary's public key.
        let primary_id = if let Some(id) = dev { id } else { 0 };
        let primary_key_file = format!("{bft_path}/.primary-{primary_id}-key.json");
        let primary_pub = read_authority_keypair_from_file(primary_key_file).unwrap().public().clone();

        // Construct the BFT consensus instance, but don't start it yet.
        let bft_execution_state = BftExecutionState::new(
            primary_pub,
            self.router.clone(),
            self.committee.clone(),
            self.consensus.clone(),
            initial_last_executed_sub_dag_index,
        );
        let bft_tx_validator = TransactionValidator(self.consensus.clone());
        let inert_bft = InertConsensusInstance::load::<N, C>(bft_execution_state, bft_tx_validator, dev)?;
        // SAFETY: must be present as the bft can only be started once quorum has been reached.
        let running_bft_consensus = inert_bft.start().await?;

        // Can't fail, but RunningConsensusInstance doesn't impl Debug, hence no unwrap.
        let _ = self.bft.set(running_bft_consensus);

        Ok(())
    }

    /// Returns the ledger.
    pub fn ledger(&self) -> &Ledger<N, C> {
        &self.ledger
    }

    /// Returns the REST server.
    pub fn rest(&self) -> &Option<Rest<N, C, Self>> {
        &self.rest
    }

    /// Return the BFT consensus handle.
    pub fn bft(&self) -> &RunningConsensusInstance<BftExecutionState<N, C>> {
        // Safe: it is used only once it's populated.
        self.bft.get().expect("Logic bug: Validator::bft didn't find a RunningConsensusInstance!")
    }

    #[cfg(feature = "test")]
    pub fn consensus(&self) -> &Consensus<N, C> {
        &self.consensus
    }

    #[cfg(feature = "test")]
    pub fn router(&self) -> &Router<N> {
        &self.router
    }
}

#[async_trait]
impl<N: Network, C: ConsensusStorage<N>> NodeInterface<N> for Validator<N, C> {
    /// Shuts down the node.
    async fn shut_down(&self) {
        info!("Shutting down...");

        // Shut down the sync pool.
        trace!("Shutting down the sync pool...");
        self.shutdown.store(true, Ordering::Relaxed);

        // Abort the tasks.
        trace!("Shutting down the validator...");
        self.handles.lock().iter().for_each(|handle| handle.abort());

        // Shut down the router.
        self.router.shut_down().await;

        // Shut down the ledger.
        trace!("Shutting down the ledger...");
        // self.ledger.shut_down().await;

        info!("Node has shut down.");
    }
}

impl<N: Network, C: ConsensusStorage<N>> Validator<N, C> {
    /// Initializes the sync pool.
    fn initialize_sync(&self) -> Result<()> {
        // Retrieve the canon locators.
        let canon_locators = crate::helpers::get_block_locators(&self.ledger)?;
        // Insert the canon locators into the sync pool.
        self.router.sync().insert_canon_locators(canon_locators).unwrap();

        // Start the sync loop.
        let validator = self.clone();
        self.handles.lock().push(tokio::spawn(async move {
            loop {
                // If the Ctrl-C handler registered the signal, stop the node.
                if validator.shutdown.load(Ordering::Relaxed) {
                    info!("Shutting down block production");
                    break;
                }

                // Sleep briefly to avoid triggering spam detection.
                tokio::time::sleep(Duration::from_secs(1)).await;

                // Prepare the block requests, if any.
                let block_requests = validator.router.sync().prepare_block_requests();
                trace!("Prepared {} block requests", block_requests.len());

                // Process the block requests.
                'outer: for (height, (hash, previous_hash, sync_ips)) in block_requests {
                    // Insert the block request into the sync pool.
                    let result =
                        validator.router.sync().insert_block_request(height, (hash, previous_hash, sync_ips.clone()));

                    // If the block request was inserted, send it to the peers.
                    if result.is_ok() {
                        // Construct the message.
                        let message =
                            Message::BlockRequest(BlockRequest { start_height: height, end_height: height + 1 });
                        // Send the message to the peers.
                        for sync_ip in sync_ips {
                            // If the send fails for any peer, remove the block request from the sync pool.
                            if validator.send(sync_ip, message.clone()).is_none() {
                                // Remove the entire block request.
                                validator.router.sync().remove_block_request(height);
                                // Break out of the loop.
                                break 'outer;
                            }
                        }
                        // Sleep for 10 milliseconds to avoid triggering spam detection.
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                }
            }
        }));
        Ok(())
    }

    /// Attempts to advance with blocks from the sync pool.
    fn advance_with_sync_blocks(&self) -> bool {
        // Retrieve the latest block height.
        let mut current_height = self.ledger.latest_height();
        // Try to advance the ledger with the sync pool.
        while let Some(block) = self.router.sync().remove_block_response(current_height + 1) {
            // Ensure the block height matches.
            if block.height() != current_height + 1 {
                warn!("Block height mismatch: expected {}, found {}", current_height + 1, block.height());
                return false;
            }
            // Check the next block.
            if let Err(error) = self.consensus.check_next_block(&block) {
                warn!("The next block ({}) is invalid - {error}", block.height());
                return false;
            }
            // Attempt to advance to the next block.
            if let Err(error) = self.consensus.advance_to_next_block(&block) {
                warn!("{error}");
                return false;
            }
            // Insert the height and hash as canon in the sync pool.
            self.router.sync().insert_canon_locator(block.height(), block.hash());
            // Increment the latest height.
            current_height += 1;
        }
        true
    }
}
