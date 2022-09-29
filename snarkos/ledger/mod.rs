// Copyright (C) 2019-2022 Aleo Systems Inc.
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
    environment::{helpers::NodeType, Environment},
    handle_dispatch_error,
    BlockDB,
    Data,
    DisconnectReason,
    Message,
    Peers,
    PeersRequest,
    PeersRouter,
    ProgramDB,
};
use snarkvm::prelude::*;

use ::time::OffsetDateTime;
use colored::Colorize;
use futures::StreamExt;
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    sync::{mpsc, oneshot},
    task,
};
use warp::{reply, Filter, Rejection, Reply};

/// Shorthand for the parent half of the `Ledger` message channel.
pub(crate) type LedgerRouter<N> = mpsc::Sender<LedgerRequest<N>>;
#[allow(unused)]
/// Shorthand for the child half of the `Ledger` message channel.
type LedgerHandler<N> = mpsc::Receiver<LedgerRequest<N>>;

///
/// An enum of requests that the `Ledger` struct processes.
///
#[derive(Debug)]
pub enum LedgerRequest<N: Network> {
    /// BlockResponse := (peer_ip, block)
    BlockResponse(SocketAddr, Block<N>),
    /// BlockRequest := (peer_ip, block_height)
    BlockRequest(SocketAddr, u32),
    /// Disconnect := (peer_ip, reason)
    Disconnect(SocketAddr, DisconnectReason),
    /// Failure := (peer_ip, failure)
    Failure(SocketAddr, String),
    /// Heartbeat
    Heartbeat,
    /// Ping := (peer_ip)
    Ping(SocketAddr),
    /// Pong := (peer_ip, block_height, node_type)
    Pong(SocketAddr, u32, Option<NodeType>),
    /// UnconfirmedBlock := (peer_ip, block)
    UnconfirmedBlock(SocketAddr, Block<N>),
    /// UnconfirmedTransaction := (peer_ip, transaction)
    UnconfirmedTransaction(SocketAddr, Transaction<N>),
}

pub(crate) type InternalLedger<N> = snarkvm::prelude::Ledger<N, BlockDB<N>, ProgramDB<N>>;
// pub(crate) type InternalLedger<N> = snarkvm::prelude::Ledger<N, BlockMemory<N>, ProgramMemory<N>>;

pub(crate) type InternalServer<N> = snarkvm::prelude::Server<N, BlockDB<N>, ProgramDB<N>>;
// pub(crate) type InternalServer<N> = snarkvm::prelude::Server<N, BlockMemory<N>, ProgramMemory<N>>;

pub type PeersState = IndexMap<SocketAddr, Option<NodeType>>;

#[allow(dead_code)]
pub struct Ledger<N: Network, E: Environment> {
    /// The ledger.
    ledger: Arc<RwLock<InternalLedger<N>>>,
    /// The ledger router of the node.
    ledger_router: LedgerRouter<N>,
    /// The server.
    server: InternalServer<N>,
    /// The account private key.
    private_key: PrivateKey<N>,
    /// The account view key.
    view_key: ViewKey<N>,
    /// The account address.
    address: Address<N>,
    /// The map of each peer to their ledger state := node_type.
    peers_state: RwLock<PeersState>,
    /// The peers router of the node.
    peers: Arc<Peers<N, E>>,
    /// The map of each peer to their failure messages := (failure_message, timestamp).
    failures: RwLock<IndexMap<SocketAddr, Vec<(String, i64)>>>,
}

impl<N: Network, E: Environment> Ledger<N, E> {
    /// Initializes a new instance of the ledger.
    pub(super) async fn new_with_genesis(
        private_key: PrivateKey<N>,
        genesis_block: Block<N>,
        peers: Arc<Peers<N, E>>,
        dev: Option<u16>,
    ) -> Result<Arc<Self>> {
        // Initialize the ledger.
        let ledger = match InternalLedger::new_with_genesis(&genesis_block, genesis_block.signature().to_address(), dev) {
            Ok(ledger) => Arc::new(RwLock::new(ledger)),
            Err(_) => {
                // Open the internal ledger.
                let ledger = InternalLedger::open(dev)?;
                // Ensure the ledger contains the correct genesis block.
                match ledger.contains_block_hash(&genesis_block.hash())? {
                    true => Arc::new(RwLock::new(ledger)),
                    false => bail!("Incorrect genesis block (run 'snarkos clean' and try again)"),
                }
            }
        };

        // Return the ledger.
        Self::from(ledger, private_key, peers).await
    }

    /// Opens an instance of the ledger.
    pub async fn load(private_key: PrivateKey<N>, peers: Arc<Peers<N, E>>, dev: Option<u16>) -> Result<Arc<Self>> {
        // Initialize the ledger.
        let ledger = Arc::new(RwLock::new(InternalLedger::open(dev)?));
        // Return the ledger.
        Self::from(ledger, private_key, peers).await
    }

    /// Initializes a new instance of the ledger.
    pub async fn from(ledger: Arc<RwLock<InternalLedger<N>>>, private_key: PrivateKey<N>, peers: Arc<Peers<N, E>>) -> Result<Arc<Self>> {
        // Derive the view key and address.
        let view_key = ViewKey::try_from(private_key)?;
        let address = Address::try_from(&view_key)?;

        // Initialize the additional routes.
        #[allow(clippy::let_and_return)]
        let additional_routes = {
            // GET /testnet3/node/address
            let get_node_address = warp::get()
                .and(warp::path!("testnet3" / "node" / "address"))
                .and(with(address))
                .and_then(|address: Address<N>| async move { Ok::<_, Rejection>(reply::json(&address.to_string())) });

            // GET /testnet3/peers/count
            let get_peers_count = warp::get()
                .and(warp::path!("testnet3" / "peers" / "count"))
                .and(with(peers.clone()))
                .and_then(get_peers_count);

            // GET /testnet3/peers/all
            let get_peers_all = warp::get()
                .and(warp::path!("testnet3" / "peers" / "all"))
                .and(with(peers.clone()))
                .and_then(get_peers_all);

            /// Returns the number of peers connected to the node.
            async fn get_peers_count<N: Network, E: Environment>(peers: Arc<Peers<N, E>>) -> Result<impl Reply, Rejection> {
                Ok(reply::json(&peers.number_of_connected_peers().await))
            }

            /// Returns the peers connected to the node.
            async fn get_peers_all<N: Network, E: Environment>(peers: Arc<Peers<N, E>>) -> Result<impl Reply, Rejection> {
                Ok(reply::json(
                    &peers.connected_peers().await.iter().map(|addr| addr.ip()).collect::<Vec<IpAddr>>(),
                ))
            }

            get_node_address.or(get_peers_count).or(get_peers_all)
        };

        // Initialize the server.
        let server = InternalServer::<N>::start(ledger.clone(), Some(additional_routes), None)?;

        // Initialize an mpsc channel for sending requests to the `Ledger` struct.
        let (ledger_router, mut ledger_handler) = mpsc::channel(1024);

        // Initialize the ledger.
        let ledger = Arc::new(Self {
            ledger,
            ledger_router,
            server,
            private_key,
            view_key,
            address,
            peers_state: Default::default(),
            failures: Default::default(),
            peers,
        });

        // Initialize the handler for the ledger.
        {
            let ledger = ledger.clone();
            let (router, handler) = oneshot::channel();
            E::resources().register_task(
                None,
                task::spawn(async move {
                    // Notify the outer function that the task is ready.
                    let _ = router.send(());
                    // Asynchronously wait for a ledger request.
                    while let Some(request) = ledger_handler.recv().await {
                        // Note: Do not wrap this call in a `task::spawn` as `BlockResponse` messages
                        // will end up being processed out of order.
                        ledger.update(request).await;
                    }
                }),
            );
            // Wait until the ledger handler is ready.
            let _ = handler.await;
        }

        // Return the ledger.
        Ok(ledger)
    }

    // TODO (raychu86): Restrict visibility.
    /// Returns the ledger.
    pub const fn ledger(&self) -> &Arc<RwLock<InternalLedger<N>>> {
        &self.ledger
    }

    /// Returns the ledger address.
    pub const fn address(&self) -> &Address<N> {
        &self.address
    }

    /// Returns the connected peers.
    pub(super) fn peers(&self) -> Arc<Peers<N, E>> {
        self.peers.clone()
    }

    /// Returns the ledger router.
    pub fn router(&self) -> LedgerRouter<N> {
        self.ledger_router.clone()
    }

    /// Returns the peers router.
    pub fn peers_router(&self) -> PeersRouter<N> {
        self.peers.router()
    }
}

impl<N: Network, E: Environment> Ledger<N, E> {
    /// Adds the given transaction to the memory pool.
    pub fn add_to_memory_pool(&self, transaction: Transaction<N>) -> Result<()> {
        self.ledger.write().add_to_memory_pool(transaction)
    }

    /// Advances the ledger to the next block.
    pub async fn advance_to_next_block(self: &Arc<Self>) -> Result<Block<N>> {
        let self_clone = self.clone();
        let next_block = task::spawn_blocking(move || {
            // Initialize an RNG.
            let rng = &mut ::rand::thread_rng();
            // Propose the next block.
            self_clone.ledger.read().propose_next_block(&self_clone.private_key, rng)
        })
        .await??;

        // Add the next block to the ledger.
        self.add_next_block(next_block.clone()).await?;

        // Serialize the block ahead of time to not do it for each peer.
        let serialized_block = Data::Object(next_block.clone()).serialize().await?;

        // Broadcast the block to all peers.
        if let Err(err) = self
            .peers()
            .router()
            .send(PeersRequest::MessagePropagate(
                *self.peers().local_ip(),
                Message::<N>::BlockBroadcast(Data::Buffer(serialized_block.clone())),
            ))
            .await
        {
            warn!("Error broadcasting BlockBroadcast to peers: {}", err);
        }

        // Return the next block.
        Ok(next_block)
    }

    /// Attempts to add the given block to the ledger.
    pub(crate) async fn add_next_block(self: &Arc<Self>, next_block: Block<N>) -> Result<()> {
        // Add the next block to the ledger.
        let self_clone = self.clone();
        if let Err(error) = task::spawn_blocking(move || self_clone.ledger.write().add_next_block(&next_block)).await? {
            // Log the error.
            warn!("{error}");
            return Err(error);
        }

        Ok(())
    }
}

impl<N: Network, E: Environment> Ledger<N, E> {
    ///
    /// Performs the given `request` to the ledger.
    /// All requests must go through this `update`, so that a unified view is preserved.
    ///
    pub(super) async fn update(self: &Arc<Self>, request: LedgerRequest<N>) {
        match request {
            LedgerRequest::BlockResponse(peer_ip, block) => {
                let block_height = block.height();
                let block_hash = block.hash();

                // Check if the block can be added to the ledger.
                if block_height == self.ledger().read().latest_height() + 1 {
                    // Attempt to add the block to the ledger.
                    match self.add_next_block(block).await {
                        Ok(_) => info!("Advanced to block {} ({})", block_height, block_hash),
                        Err(err) => warn!("Failed to process block {} (height: {}): {:?}", block_hash, block_height, err),
                    };

                    // TODO (raychu86): Remove this. Currently used for naive sync.
                    // Send a ping.
                    if let Err(err) = self
                        .peers_router()
                        .send(PeersRequest::MessageSend(peer_ip, Message::<N>::Ping))
                        .await
                    {
                        warn!("[Ping] {}", err);
                    }
                } else {
                    trace!("Skipping block {} (height: {})", block_hash, block_height);
                }
            }
            LedgerRequest::BlockRequest(peer_ip, height) => {
                let latest_height = self.ledger().read().latest_height();
                if height > latest_height {
                    trace!("Peer requested block {height}, which is greater than the current height {latest_height}");
                } else {
                    let block = match self.ledger().read().get_block(height) {
                        Ok(block) => block,
                        Err(err) => {
                            warn!("Failed to retrieve block {height} from the ledger: {err}");
                            return;
                        }
                    };

                    if let Err(err) = self
                        .peers_router()
                        .send(PeersRequest::MessageSend(peer_ip, Message::BlockResponse(Data::Object(block))))
                        .await
                    {
                        warn!("[BlockResponse] {}", err);
                    }
                }
            }
            LedgerRequest::Disconnect(peer_ip, reason) => {
                self.disconnect(peer_ip, reason).await;
            }
            LedgerRequest::Failure(peer_ip, failure) => {
                self.add_failure(peer_ip, failure).await;
            }
            LedgerRequest::Heartbeat => {}
            LedgerRequest::Ping(peer_ip) => {
                // Send a `Pong` message to the peer.
                let latest_height = self.ledger().read().latest_height();
                if let Err(error) = self
                    .peers_router()
                    .send(PeersRequest::MessageSend(peer_ip, Message::<N>::Pong(latest_height)))
                    .await
                {
                    warn!("[Ping] {}", error);
                }
            }
            LedgerRequest::Pong(peer_ip, height, node_type) => {
                // Ensure the peer has been initialized in the ledger.
                self.initialize_peer(peer_ip, node_type).await;

                // If the peer is ahead, ask for next block.
                let latest_height = self.ledger().read().latest_height();
                if height > latest_height {
                    if let Err(err) = self
                        .peers_router()
                        .send(PeersRequest::MessageSend(peer_ip, Message::<N>::BlockRequest(latest_height + 1)))
                        .await
                    {
                        warn!("[BlockRequest] {}", err);
                    }
                }
            }
            LedgerRequest::UnconfirmedBlock(peer_ip, block) => {
                let block_height = block.height();
                let block_hash = block.hash();

                // Attempt to add the block to the ledger.
                match self.add_next_block(block.clone()).await {
                    Ok(_) => {
                        info!("Advanced to block {} ({})", block_height, block_hash);

                        // Broadcast block to all peers except the sender.
                        let message = Message::BlockBroadcast(Data::Object(block));
                        if let Err(error) = self.peers_router().send(PeersRequest::MessagePropagate(peer_ip, message)).await {
                            warn!("[UnconfirmedBlock] {}", error);
                        }
                    }
                    Err(err) => {
                        trace!("Failed to process block {} (height: {}): {:?}", block_hash, block_height, err);
                    }
                }
            }
            LedgerRequest::UnconfirmedTransaction(peer_ip, transaction) => {
                // Attempt to insert the transaction into the mempool.
                match self.add_to_memory_pool(transaction.clone()) {
                    Ok(_) => {
                        // Broadcast transaction to all peers except the sender.
                        let message = Message::TransactionBroadcast(Data::Object(transaction));
                        if let Err(error) = self.peers_router().send(PeersRequest::MessagePropagate(peer_ip, message)).await {
                            warn!("[UnconfirmedTransaction] {}", error);
                        }
                    }
                    Err(err) => {
                        trace!("Failed to add transaction {} to mempool: {:?}", transaction.id(), err);
                    }
                }
            }
        }
    }

    ///
    /// Disconnects the given peer from the ledger.
    ///
    async fn disconnect(&self, peer_ip: SocketAddr, reason: DisconnectReason) {
        info!("Disconnecting from {} ({:?})", peer_ip, reason);
        // Remove all entries of the peer from the ledger.
        self.remove_peer(&peer_ip).await;
        // Send a `Disconnect` message to the peer.
        if let Err(error) = self
            .peers_router()
            .send(PeersRequest::MessageSend(peer_ip, Message::Disconnect(reason)))
            .await
        {
            warn!("[Disconnect] {}", error);
        }
        // Route a `PeerDisconnected` to the peers.
        if let Err(error) = self.peers_router().send(PeersRequest::PeerDisconnected(peer_ip)).await {
            warn!("[PeerDisconnected] {}", error);
        }
    }

    ///
    /// Disconnects and restricts the given peer from the ledger.
    ///
    #[allow(dead_code)]
    async fn disconnect_and_restrict(&self, peer_ip: SocketAddr, reason: DisconnectReason) {
        info!("Disconnecting and restricting {} ({:?})", peer_ip, reason);
        // Remove all entries of the peer from the ledger.
        self.remove_peer(&peer_ip).await;
        // Send a `Disconnect` message to the peer.
        if let Err(error) = self
            .peers_router()
            .send(PeersRequest::MessageSend(peer_ip, Message::Disconnect(reason)))
            .await
        {
            warn!("[Disconnect] {}", error);
        }
        // Route a `PeerRestricted` to the peers.
        if let Err(error) = self.peers_router().send(PeersRequest::PeerRestricted(peer_ip)).await {
            warn!("[PeerRestricted] {}", error);
        }
    }

    ///
    /// Adds an entry for the given peer IP to every data structure in `State`.
    ///
    async fn initialize_peer(&self, peer_ip: SocketAddr, node_type: Option<NodeType>) {
        // Since the peer state already existing is the most probable scenario,
        // use a read() first to avoid using write() if possible.
        let peer_state_exists = self.peers_state.read().contains_key(&peer_ip);

        if !peer_state_exists {
            self.peers_state.write().entry(peer_ip).or_insert(node_type);
            self.failures.write().entry(peer_ip).or_insert_with(Default::default);
        }
    }

    ///
    /// Removes the entry for the given peer IP from every data structure in `State`.
    ///
    async fn remove_peer(&self, peer_ip: &SocketAddr) {
        self.peers_state.write().remove(peer_ip);
        self.failures.write().remove(peer_ip);
    }

    ///
    /// Adds the given failure message to the specified peer IP.
    ///
    async fn add_failure(&self, peer_ip: SocketAddr, failure: String) {
        trace!("Adding failure for {}: {}", peer_ip, failure);
        match self.failures.write().get_mut(&peer_ip) {
            Some(failures) => failures.push((failure, OffsetDateTime::now_utc().unix_timestamp())),
            None => error!("Missing failure entry for {}", peer_ip),
        };
    }
}

// Internal operations.
impl<N: Network, E: Environment> Ledger<N, E> {
    /// Returns the unspent records.
    pub fn find_unspent_records(&self) -> Result<IndexMap<Field<N>, Record<N, Plaintext<N>>>> {
        // Fetch the unspent records.
        let records = self
            .ledger
            .read()
            .find_records(&self.view_key, RecordsFilter::Unspent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>();
        // Return the unspent records.
        Ok(records)
    }

    /// Returns the spent records.
    pub fn find_spent_records(&self) -> Result<IndexMap<Field<N>, Record<N, Plaintext<N>>>> {
        // Fetch the unspent records.
        let records = self
            .ledger
            .read()
            .find_records(&self.view_key, RecordsFilter::Spent)?
            .filter(|(_, record)| !record.gates().is_zero())
            .collect::<IndexMap<_, _>>();
        // Return the unspent records.
        Ok(records)
    }

    /// Creates a deploy transaction.
    pub fn create_deploy(&self, program: &Program<N>, additional_fee: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records()?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Prepare the additional fee.
        let credits = records.values().max_by(|a, b| (**a.gates()).cmp(&**b.gates())).unwrap().clone();
        ensure!(
            ***credits.gates() >= additional_fee,
            "The additional fee is more than the record balance."
        );

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();
        // Deploy.
        let transaction = Transaction::deploy(self.ledger.read().vm(), &self.private_key, program, (credits, additional_fee), rng)?;
        // Verify.
        assert!(self.ledger.read().vm().verify(&transaction));
        // Return the transaction.
        Ok(transaction)
    }

    /// Creates a transfer transaction.
    pub fn create_transfer(&self, to: &Address<N>, amount: u64) -> Result<Transaction<N>> {
        // Fetch the unspent records.
        let records = self.find_unspent_records()?;
        ensure!(!records.len().is_zero(), "The Aleo account has no records to spend.");

        // Initialize an RNG.
        let rng = &mut ::rand::thread_rng();

        // Create a new transaction.
        Transaction::execute(
            self.ledger.read().vm(),
            &self.private_key,
            &ProgramID::from_str("credits.aleo")?,
            Identifier::from_str("transfer")?,
            &[
                Value::Record(records.values().next().unwrap().clone()),
                Value::from_str(&format!("{to}"))?,
                Value::from_str(&format!("{amount}u64"))?,
            ],
            None,
            rng,
        )
    }
}

// Internal operations.
impl<N: Network, E: Environment> Ledger<N, E> {
    /// Syncs the ledger with the network.
    pub(crate) async fn initial_sync_with_network(self: &Arc<Self>, leader_ip: IpAddr) -> Result<()> {
        /// The number of concurrent requests with the network.
        const CONCURRENT_REQUESTS: usize = 100;
        /// Url to fetch the blocks from.
        const TARGET_URL: &str = "https://vm.aleo.org/testnet3/block/testnet3/";

        // Fetch the ledger height.
        let ledger_height = self.ledger.read().latest_height();

        // Fetch the latest height.
        let latest_height = reqwest::get(format!("http://{leader_ip}/testnet3/latest/height"))
            .await?
            .text()
            .await?
            .parse::<u32>()?;

        // Start a timer.
        let timer = std::time::Instant::now();

        // Sync the ledger to the latest block height.
        if latest_height > ledger_height + 1 {
            futures::stream::iter((ledger_height + 1)..=latest_height)
                .map(|height| {
                    trace!("Requesting block {height} of {latest_height}");

                    // Download the block with an exponential backoff retry policy.
                    handle_dispatch_error(move || async move {
                        // Get the URL for the block download.
                        let block_url = format!("{TARGET_URL}{height}.block");

                        // Fetch the bytes from the given url
                        let block_bytes = reqwest::get(block_url).await?.bytes().await?;

                        // Parse the block.
                        let block = task::spawn_blocking(move || Block::from_bytes_le(&block_bytes)).await.unwrap()?;

                        std::future::ready(Ok(block)).await
                    })
                })
                .buffered(CONCURRENT_REQUESTS)
                .for_each(|block| async {
                    let block = block.unwrap();
                    // Use blocking tasks, as deserialization and adding blocks are expensive operations.
                    let self_clone = self.clone();

                    task::spawn_blocking(move || {
                        // Add the block to the ledger.
                        self_clone.ledger.write().add_next_block(&block).unwrap();

                        // Retrieve the current height.
                        let height = block.height();
                        // Compute the percentage completed.
                        let percentage = height * 100 / latest_height;
                        // Compute the time remaining (in millis).
                        let millis_per_block = (timer.elapsed().as_millis()) / (height - ledger_height) as u128;
                        let time_remaining = (latest_height - height) as u128 * millis_per_block;
                        // Prepare the estimate message (in secs).
                        let estimate = format!("(est. {} minutes remaining)", time_remaining / (60 * 1000));
                        // Log the progress.
                        info!(
                            "Synced up to block {height} of {latest_height} - {percentage}% complete {}",
                            estimate.dimmed()
                        );
                    })
                    .await
                    .unwrap();
                })
                .await;
        }

        Ok(())
    }
}
