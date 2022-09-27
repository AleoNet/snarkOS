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

use crate::{handle_dispatch_error, BlockDB, Data, ProgramDB};
use snarkvm::prelude::*;

use colored::Colorize;
use futures::StreamExt;
use indexmap::IndexMap;
use parking_lot::RwLock;
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::task;
use warp::{reply, Filter, Rejection, Reply};

pub(crate) type InternalLedger<N> = snarkvm::prelude::Ledger<N, BlockDB<N>, ProgramDB<N>>;
// pub(crate) type InternalLedger<N> = snarkvm::prelude::Ledger<N, BlockMemory<N>, ProgramMemory<N>>;

pub(crate) type InternalServer<N> = snarkvm::prelude::Server<N, BlockDB<N>, ProgramDB<N>>;
// pub(crate) type InternalServer<N> = snarkvm::prelude::Server<N, BlockMemory<N>, ProgramMemory<N>>;

pub(crate) type Peers<N> = Arc<RwLock<IndexMap<SocketAddr, crate::Sender<N>>>>;

#[allow(dead_code)]
pub struct Ledger<N: Network> {
    /// The ledger.
    ledger: Arc<RwLock<InternalLedger<N>>>,
    /// The server.
    server: InternalServer<N>,
    /// The peers.
    peers: Peers<N>,
    /// The account private key.
    private_key: PrivateKey<N>,
    /// The account view key.
    view_key: ViewKey<N>,
    /// The account address.
    address: Address<N>,
}

impl<N: Network> Ledger<N> {
    /// Initializes a new instance of the ledger.
    pub(super) fn new_with_genesis(private_key: PrivateKey<N>, genesis_block: Block<N>, dev: Option<u16>) -> Result<Arc<Self>> {
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
        Self::from(ledger, private_key)
    }

    /// Opens an instance of the ledger.
    pub fn load(private_key: PrivateKey<N>, dev: Option<u16>) -> Result<Arc<Self>> {
        // Initialize the ledger.
        let ledger = Arc::new(RwLock::new(InternalLedger::open(dev)?));
        // Return the ledger.
        Self::from(ledger, private_key)
    }

    /// Initializes a new instance of the ledger.
    pub fn from(ledger: Arc<RwLock<InternalLedger<N>>>, private_key: PrivateKey<N>) -> Result<Arc<Self>> {
        // Derive the view key and address.
        let view_key = ViewKey::try_from(private_key)?;
        let address = Address::try_from(&view_key)?;

        // Initialize the peers.
        let peers: Peers<N> = Default::default();

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
            async fn get_peers_count<N: Network>(peers: Peers<N>) -> Result<impl Reply, Rejection> {
                Ok(reply::json(&peers.read().len()))
            }

            /// Returns the peers connected to the node.
            async fn get_peers_all<N: Network>(peers: Peers<N>) -> Result<impl Reply, Rejection> {
                Ok(reply::json(&peers.read().keys().map(|addr| addr.ip()).collect::<Vec<IpAddr>>()))
            }

            get_node_address.or(get_peers_count).or(get_peers_all)
        };

        // Initialize the server.
        let server = InternalServer::<N>::start(ledger.clone(), Some(additional_routes), None)?;

        // Return the ledger.
        Ok(Arc::new(Self {
            ledger,
            server,
            peers,
            private_key,
            view_key,
            address,
        }))
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
    pub(super) const fn peers(&self) -> &Peers<N> {
        &self.peers
    }
}

impl<N: Network> Ledger<N> {
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
        let peers = self.peers().read().clone();
        for (_, sender) in peers.iter() {
            let _ = sender
                .send(crate::Message::<N>::BlockBroadcast(Data::Buffer(serialized_block.clone())))
                .await;
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

// Internal operations.
impl<N: Network> Ledger<N> {
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
impl<N: Network> Ledger<N> {
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
