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

use crate::Ledger;
use anyhow::Result;
use core::marker::PhantomData;
use indexmap::IndexMap;
use serde_json::json;
use snarkvm::prelude::{AdditionalFee, Deployment, Execution, Field, Network, RecordsFilter, Transaction, ViewKey, U64};
use std::sync::Arc;
use tokio::{sync::mpsc, task::JoinHandle};
use warp::{http::StatusCode, reject, reply, Filter, Rejection, Reply};

/// An enum of error handlers for the server.
#[derive(Debug)]
enum ServerError {
    Request(String),
}

impl reject::Reject for ServerError {}

/// A trait to unwrap a `Result` or `Reject`.
pub trait OrReject<T> {
    /// Returns the result if it is successful, otherwise returns a rejection.
    fn or_reject(self) -> Result<T, Rejection>;
}

impl<T> OrReject<T> for anyhow::Result<T> {
    /// Returns the result if it is successful, otherwise returns a rejection.
    fn or_reject(self) -> Result<T, Rejection> {
        self.map_err(|e| reject::custom(ServerError::Request(e.to_string())))
    }
}

/// A middleware to include the given item in the handler.
fn with<T: Clone + Send>(item: T) -> impl Filter<Extract = (T,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || item.clone())
}

/// Shorthand for the parent half of the `Ledger` message channel.
pub type LedgerSender<N> = mpsc::Sender<LedgerRequest<N>>;
/// Shorthand for the child half of the `Ledger` message channel.
pub type LedgerReceiver<N> = mpsc::Receiver<LedgerRequest<N>>;

/// An enum of requests that the `Ledger` struct processes.
#[derive(Debug)]
pub enum LedgerRequest<N: Network> {
    TransactionBroadcast(Transaction<N>),
}

/// A server for the ledger.
#[allow(dead_code)]
pub struct Server<N: Network> {
    /// The ledger.
    ledger: Arc<Ledger<N>>,
    /// The ledger sender.
    ledger_sender: LedgerSender<N>,
    /// The server handles.
    handles: Vec<JoinHandle<()>>,
    /// PhantomData.
    _phantom: PhantomData<N>,
}

impl<N: Network> Server<N> {
    /// Initializes a new instance of the server.
    pub fn start(ledger: Arc<Ledger<N>>) -> Result<Self> {
        // Initialize a channel to send requests to the ledger.
        let (ledger_sender, ledger_receiver) = mpsc::channel(64);

        // Initialize a vector for the server handles.
        let mut handles = Vec::new();

        // Initialize the routes.
        let routes = Self::routes(ledger.clone(), ledger_sender.clone());

        // Spawn the server.
        handles.push(tokio::spawn(async move {
            // Start the server.
            warp::serve(routes).run(([0, 0, 0, 0], 80)).await;
        }));

        // Spawn the ledger handler.
        handles.push(Self::start_handler(ledger.clone(), ledger_receiver));

        Ok(Self {
            ledger,
            ledger_sender,
            handles,
            _phantom: PhantomData,
        })
    }

    /// Initializes the routes, given the ledger and ledger sender.
    fn routes(ledger: Arc<Ledger<N>>, ledger_sender: LedgerSender<N>) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        // GET /testnet3/latest/height
        let latest_height = warp::get()
            .and(warp::path!("testnet3" / "latest" / "height"))
            .and(with(ledger.clone()))
            .and_then(Self::latest_height);

        // GET /testnet3/latest/hash
        let latest_hash = warp::get()
            .and(warp::path!("testnet3" / "latest" / "hash"))
            .and(with(ledger.clone()))
            .and_then(Self::latest_hash);

        // GET /testnet3/latest/block
        let latest_block = warp::get()
            .and(warp::path!("testnet3" / "latest" / "block"))
            .and(with(ledger.clone()))
            .and_then(Self::latest_block);

        // GET /testnet3/block/{height}
        let get_block = warp::get()
            .and(warp::path!("testnet3" / "block" / u32))
            .and(with(ledger.clone()))
            .and_then(Self::get_block);

        // GET /testnet3/statePath/{commitment}
        let state_path = warp::get()
            .and(warp::path!("testnet3" / "statePath"))
            .and(warp::body::content_length_limit(128))
            .and(warp::body::json())
            .and(with(ledger.clone()))
            .and_then(Self::state_path);

        // GET /testnet3/records/all
        let records_all = warp::get()
            .and(warp::path!("testnet3" / "records" / "all"))
            .and(warp::body::content_length_limit(128))
            .and(warp::body::json())
            .and(with(ledger.clone()))
            .and_then(Self::records_all);

        // GET /testnet3/records/spent
        let records_spent = warp::get()
            .and(warp::path!("testnet3" / "records" / "spent"))
            .and(warp::body::content_length_limit(128))
            .and(warp::body::json())
            .and(with(ledger.clone()))
            .and_then(Self::records_spent);

        // GET /testnet3/records/unspent
        let records_unspent = warp::get()
            .and(warp::path!("testnet3" / "records" / "unspent"))
            .and(warp::body::content_length_limit(128))
            .and(warp::body::json())
            .and(with(ledger.clone()))
            .and_then(Self::records_unspent);

        // GET /testnet3/peers/count
        let peers_count = warp::get()
            .and(warp::path!("testnet3" / "peers" / "count"))
            .and(with(ledger.clone()))
            .and_then(Self::peers_count);

        // GET /testnet3/peers/all
        let peers_all = warp::get()
            .and(warp::path!("testnet3" / "peers" / "all"))
            .and(with(ledger.clone()))
            .and_then(Self::peers_all);

        // GET /testnet3/transactions/{height}
        let get_transactions = warp::get()
            .and(warp::path!("testnet3" / "transactions" / u32))
            .and(with(ledger.clone()))
            .and_then(Self::get_transactions);

        // GET /testnet3/transaction/{id}
        let get_transaction = warp::get()
            .and(warp::path!("testnet3" / "transaction" / ..))
            .and(warp::path::param::<N::TransactionID>())
            .and(warp::path::end())
            .and(with(ledger.clone()))
            .and_then(Self::get_transaction);

        // POST /testnet3/transaction/broadcast
        let transaction_broadcast = warp::post()
            .and(warp::path!("testnet3" / "transaction" / "broadcast"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(ledger_sender.clone()))
            .and_then(Self::transaction_broadcast);

        // GET /testnet3/program/program_id
        let get_program = warp::get()
            .and(warp::path!("testnet3" / "program" / u32))
            .and(warp::body::content_length_limit(128))
            .and(with(ledger.clone()))
            .and_then(Self::get_program);

        // POST /testnet3/deploy
        let deploy_program = warp::post()
            .and(warp::path!("testnet3" / "deploy"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(ledger.clone()))
            .and(with(ledger_sender.clone()))
            .and_then(Self::deploy_program);

        // POST /testnet3/execute
        let execute_program = warp::post()
            .and(warp::path!("testnet3" / "execute"))
            .and(warp::body::content_length_limit(10 * 1024 * 1024))
            .and(warp::body::json())
            .and(with(ledger))
            .and(with(ledger_sender))
            .and_then(Self::execute_program);

        // Return the list of routes.
        latest_height
            .or(latest_hash)
            .or(latest_block)
            .or(get_block)
            .or(state_path)
            .or(records_all)
            .or(records_spent)
            .or(records_unspent)
            .or(peers_count)
            .or(peers_all)
            .or(get_transactions)
            .or(get_transaction)
            .or(transaction_broadcast)
            .or(deploy_program)
            .or(execute_program)
            .or(get_program)
    }

    /// Initializes a ledger handler.
    fn start_handler(ledger: Arc<Ledger<N>>, mut ledger_receiver: LedgerReceiver<N>) -> JoinHandle<()> {
        tokio::spawn(async move {
            while let Some(request) = ledger_receiver.recv().await {
                match request {
                    LedgerRequest::TransactionBroadcast(transaction) => {
                        let transaction_id = transaction.id();
                        match ledger.add_to_memory_pool(transaction) {
                            Ok(()) => trace!("✉️ Added transaction '{transaction_id}' to the memory pool"),
                            Err(error) => {
                                warn!("⚠️ Failed to add transaction '{transaction_id}' to the memory pool: {error}")
                            }
                        }
                    }
                };
            }
        })
    }
}

impl<N: Network> Server<N> {
    /// Returns the latest block height.
    async fn latest_height(ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().latest_height()))
    }

    /// Returns the latest block hash.
    async fn latest_hash(ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().latest_hash()))
    }

    /// Returns the latest block.
    async fn latest_block(ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().latest_block().or_reject()?))
    }

    /// Returns the block for the given block height.
    async fn get_block(height: u32, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().get_block(height).or_reject()?))
    }

    /// Returns the program with the given id
    async fn get_program(program_id: u32, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().get_program(program_id).or_reject()?))
    }

    /// Returns the state path for the given commitment.
    async fn state_path(commitment: Field<N>, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().to_state_path(&commitment).or_reject()?))
    }

    /// Returns all of the records for the given view key.
    async fn records_all(view_key: ViewKey<N>, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        // Fetch the records using the view key.
        let records: IndexMap<_, _> = ledger
            .ledger
            .read()
            .find_records(&view_key, RecordsFilter::All)
            .or_reject()?
            .collect();
        println!("Records:\n{:#?}", records);
        // Return the records.
        Ok(reply::with_status(reply::json(&records), StatusCode::OK))
    }

    /// Returns the spent records for the given view key.
    async fn records_spent(view_key: ViewKey<N>, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        // Fetch the records using the view key.
        let records = ledger
            .ledger
            .read()
            .find_records(&view_key, RecordsFilter::Spent)
            .or_reject()?
            .collect::<IndexMap<_, _>>();
        println!("Records:\n{:#?}", records);
        // Return the records.
        Ok(reply::with_status(reply::json(&records), StatusCode::OK))
    }

    /// Returns the unspent records for the given view key.
    async fn records_unspent(view_key: ViewKey<N>, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        // Fetch the records using the view key.
        let records = ledger
            .ledger
            .read()
            .find_records(&view_key, RecordsFilter::Unspent)
            .or_reject()?
            .collect::<IndexMap<_, _>>();
        println!("Records:\n{:#?}", records);
        // Return the records.
        Ok(reply::with_status(reply::json(&records), StatusCode::OK))
    }

    /// Returns the number of peers connected to the node.
    async fn peers_count(ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.peers.read().len()))
    }

    /// Returns the peers connected to the node.
    async fn peers_all(ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(
            &ledger.peers.read().keys().cloned().collect::<Vec<std::net::SocketAddr>>(),
        ))
    }

    /// Returns the transactions for the given block height.
    async fn get_transactions(height: u32, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().get_transactions(height).or_reject()?))
    }

    /// Returns the transaction for the given transaction ID.
    async fn get_transaction(transaction_id: N::TransactionID, ledger: Arc<Ledger<N>>) -> Result<impl Reply, Rejection> {
        Ok(reply::json(&ledger.ledger.read().get_transaction(transaction_id).or_reject()?))
    }

    /// Broadcasts the transaction to the ledger.
    async fn transaction_broadcast(transaction: Transaction<N>, ledger_sender: LedgerSender<N>) -> Result<impl Reply, Rejection> {
        // Send the transaction to the ledger.
        match ledger_sender.send(LedgerRequest::TransactionBroadcast(transaction)).await {
            Ok(()) => Ok("OK"),
            Err(error) => Err(reject::custom(ServerError::Request(format!("{error}")))),
        }
    }

    /// Send a program deployment transaction to the ledger
    async fn deploy_program(
        deployment: Deployment<N>,
        ledger: Arc<Ledger<N>>,
        ledger_sender: LedgerSender<N>,
    ) -> Result<impl Reply, Rejection> {
        let additional_fee = Self::execute_additional_fee(ledger)?;
        let transaction = Transaction::from_deployment(deployment.clone(), additional_fee).or_reject()?;
        match ledger_sender.send(LedgerRequest::TransactionBroadcast(transaction)).await {
            Ok(()) => Ok(reply::with_status(
                reply::json(&json!({ "deployment": deployment })),
                StatusCode::OK,
            )),
            Err(error) => Err(reject::custom(ServerError::Request(format!("{error}")))),
        }
    }

    /// Send a program execution transaction to the ledger
    async fn execute_program(
        execution: Execution<N>,
        ledger: Arc<Ledger<N>>,
        ledger_sender: LedgerSender<N>,
    ) -> Result<impl Reply, Rejection> {
        let additional_fee = Self::execute_additional_fee(ledger)?;
        let transaction = Transaction::from_execution(execution.clone(), Some(additional_fee)).or_reject()?;
        match ledger_sender.send(LedgerRequest::TransactionBroadcast(transaction)).await {
            Ok(()) => Ok(reply::with_status(reply::json(&json!({ "execution": execution })), StatusCode::OK)),
            Err(error) => Err(reject::custom(ServerError::Request(format!("{error}")))),
        }
    }

    /// Spends the record with the smallest value with at least `1 gate` as a transaction fee.
    fn execute_additional_fee(ledger: Arc<Ledger<N>>) -> Result<AdditionalFee<N>, Rejection> {
        let records = ledger.find_unspent_records().or_reject()?;

        // Get smallest record with at least 1 gate
        let one_gate = U64::new(1u64);
        let credits = records
            .values()
            .filter(|record| **record.gates() >= one_gate)
            .min_by(|a, b| (**a.gates()).cmp(&**b.gates()))
            .unwrap()
            .clone();

        let additional_fee_in_gates = credits.gates().clone();

        // Create the additional fee
        ledger
            .ledger
            .read()
            .vm()
            .execute_additional_fee(&ledger.private_key, credits, **additional_fee_in_gates, &mut rand::thread_rng())
            .map(|(_, additional_fee)| additional_fee)
            .or_reject()
    }
}
