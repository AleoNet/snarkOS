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

#![forbid(unsafe_code)]

#[macro_use]
extern crate tracing;

use snarkos_account::Account;
use snarkos_node_bft::{
    helpers::{
        fmt_id,
        init_consensus_channels,
        now_nanos,
        ConsensusReceiver,
        PrimaryReceiver,
        PrimarySender,
        Storage as NarwhalStorage,
    },
    spawn_blocking,
    BFT,
    MAX_GC_ROUNDS,
    MAX_TRANSMISSIONS_PER_BATCH,
};
use snarkos_node_bft_ledger_service::LedgerService;
use snarkos_node_bft_storage_service::BFTPersistentStorage;
use snarkvm::{
    ledger::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        narwhal::{Data, Subdag, Transmission, TransmissionID},
    },
    prelude::*,
};

use anyhow::Result;
use colored::Colorize;
use indexmap::{IndexMap, IndexSet};
use lru::LruCache;
use parking_lot::Mutex;
use std::{collections::BTreeMap, future::Future, net::SocketAddr, num::NonZeroUsize, sync::Arc};
use tokio::{
    sync::{oneshot, OnceCell},
    task::JoinHandle,
};

const MAX_TX_QUEUE_SIZE: usize = 1 << 30; // in bytes

type PriorityFee<N> = U64<N>;
type Timestamp = i128;
// Transactions are sorted by PriorityFee first, oldest Timestamp second, and TransactionID last.
type TransactionKey<N> = (PriorityFee<N>, Timestamp, Field<N>);

#[derive(Clone)]
pub struct Consensus<N: Network> {
    /// The ledger.
    ledger: Arc<dyn LedgerService<N>>,
    /// The BFT.
    bft: BFT<N>,
    /// The primary sender.
    primary_sender: Arc<OnceCell<PrimarySender<N>>>,
    /// The unconfirmed solutions queue.
    solutions_queue: Arc<Mutex<IndexMap<PuzzleCommitment<N>, ProverSolution<N>>>>,
    /// The unconfirmed transactions queue.
    transactions_queue: Arc<Mutex<BTreeMap<TransactionKey<N>, Transaction<N>>>>,
    /// Set of transactions in queue.
    transactions_in_queue: Arc<Mutex<IndexSet<N::TransactionID>>>,
    /// The recently-seen unconfirmed solutions.
    seen_solutions: Arc<Mutex<LruCache<PuzzleCommitment<N>, ()>>>,
    /// The recently-seen unconfirmed transactions.
    seen_transactions: Arc<Mutex<LruCache<N::TransactionID, ()>>>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network> Consensus<N> {
    /// Initializes a new instance of consensus.
    pub fn new(
        account: Account<N>,
        ledger: Arc<dyn LedgerService<N>>,
        ip: Option<SocketAddr>,
        trusted_validators: &[SocketAddr],
        dev: Option<u16>,
    ) -> Result<Self> {
        // Initialize the Narwhal transmissions.
        let transmissions = Arc::new(BFTPersistentStorage::open(dev)?);
        // Initialize the Narwhal storage.
        let storage = NarwhalStorage::new(ledger.clone(), transmissions, MAX_GC_ROUNDS);
        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger.clone(), ip, trusted_validators, dev)?;
        // Return the consensus.
        Ok(Self {
            ledger,
            bft,
            primary_sender: Default::default(),
            solutions_queue: Default::default(),
            transactions_queue: Default::default(),
            transactions_in_queue: Default::default(),
            seen_solutions: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1 << 16).unwrap()))),
            seen_transactions: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1 << 16).unwrap()))),
            handles: Default::default(),
        })
    }

    /// Run the consensus instance.
    pub async fn run(&mut self, primary_sender: PrimarySender<N>, primary_receiver: PrimaryReceiver<N>) -> Result<()> {
        info!("Starting the consensus instance...");
        // Set the primary sender.
        self.primary_sender.set(primary_sender.clone()).expect("Primary sender already set");

        // First, initialize the consensus channels.
        let (consensus_sender, consensus_receiver) = init_consensus_channels();
        // Then, start the consensus handlers.
        self.start_handlers(consensus_receiver);
        // Lastly, the consensus.
        self.bft.run(Some(consensus_sender), primary_sender, primary_receiver).await?;
        Ok(())
    }

    /// Returns the ledger.
    pub const fn ledger(&self) -> &Arc<dyn LedgerService<N>> {
        &self.ledger
    }

    /// Returns the BFT.
    pub const fn bft(&self) -> &BFT<N> {
        &self.bft
    }

    /// Returns the primary sender.
    pub fn primary_sender(&self) -> &PrimarySender<N> {
        self.primary_sender.get().expect("Primary sender not set")
    }
}

impl<N: Network> Consensus<N> {
    /// Returns the number of unconfirmed transmissions.
    pub fn num_unconfirmed_transmissions(&self) -> usize {
        self.bft.num_unconfirmed_transmissions()
    }

    /// Returns the number of unconfirmed ratifications.
    pub fn num_unconfirmed_ratifications(&self) -> usize {
        self.bft.num_unconfirmed_ratifications()
    }

    /// Returns the number of solutions.
    pub fn num_unconfirmed_solutions(&self) -> usize {
        self.bft.num_unconfirmed_solutions()
    }

    /// Returns the number of unconfirmed transactions.
    pub fn num_unconfirmed_transactions(&self) -> usize {
        self.bft.num_unconfirmed_transactions()
    }
}

impl<N: Network> Consensus<N> {
    /// Returns the unconfirmed transmission IDs.
    pub fn unconfirmed_transmission_ids(&self) -> impl '_ + Iterator<Item = TransmissionID<N>> {
        self.bft.unconfirmed_transmission_ids()
    }

    /// Returns the unconfirmed transmissions.
    pub fn unconfirmed_transmissions(&self) -> impl '_ + Iterator<Item = (TransmissionID<N>, Transmission<N>)> {
        self.bft.unconfirmed_transmissions()
    }

    /// Returns the unconfirmed solutions.
    pub fn unconfirmed_solutions(&self) -> impl '_ + Iterator<Item = (PuzzleCommitment<N>, Data<ProverSolution<N>>)> {
        self.bft.unconfirmed_solutions()
    }

    /// Returns the unconfirmed transactions.
    pub fn unconfirmed_transactions(&self) -> impl '_ + Iterator<Item = (N::TransactionID, Data<Transaction<N>>)> {
        self.bft.unconfirmed_transactions()
    }
}

impl<N: Network> Consensus<N> {
    /// Adds the given unconfirmed solution to the memory pool.
    pub async fn add_unconfirmed_solution(&self, solution: ProverSolution<N>) -> Result<()> {
        // Process the unconfirmed solution.
        {
            let solution_id = solution.commitment();

            // Check if the transaction was recently seen.
            if self.seen_solutions.lock().put(solution_id, ()).is_some() {
                // If the transaction was recently seen, return early.
                return Ok(());
            }
            // Check if the solution already exists in the ledger.
            if self.ledger.contains_transmission(&TransmissionID::from(solution_id))? {
                bail!("Solution '{}' exists in the ledger {}", fmt_id(solution_id), "(skipping)".dimmed());
            }
            // Add the solution to the memory pool.
            trace!("Received unconfirmed solution '{}' in the queue", fmt_id(solution_id));
            if self.solutions_queue.lock().insert(solution_id, solution).is_some() {
                bail!("Solution '{}' exists in the memory pool", fmt_id(solution_id));
            }
        }

        // If the memory pool of this node is full, return early.
        let num_unconfirmed = self.num_unconfirmed_transmissions();
        if num_unconfirmed > N::MAX_SOLUTIONS || num_unconfirmed > MAX_TRANSMISSIONS_PER_BATCH {
            return Ok(());
        }
        // Retrieve the solutions.
        let solutions = {
            // Determine the available capacity.
            let capacity = N::MAX_SOLUTIONS.saturating_sub(num_unconfirmed);
            // Acquire the lock on the queue.
            let mut queue = self.solutions_queue.lock();
            // Determine the number of solutions to send.
            let num_solutions = queue.len().min(capacity);
            // Drain the solutions from the queue.
            queue.drain(..num_solutions).collect::<Vec<_>>()
        };
        // Iterate over the solutions.
        for (_, solution) in solutions.into_iter() {
            let solution_id = solution.commitment();
            trace!("Adding unconfirmed solution '{}' to the memory pool...", fmt_id(solution_id));
            // Send the unconfirmed solution to the primary.
            if let Err(e) = self.primary_sender().send_unconfirmed_solution(solution_id, Data::Object(solution)).await {
                warn!("Failed to add unconfirmed solution '{}' to the memory pool - {e}", fmt_id(solution_id));
            }
        }
        Ok(())
    }

    /// Adds the given unconfirmed transaction to the memory pool.
    pub async fn add_unconfirmed_transaction(&self, transaction: Transaction<N>) -> Result<()> {
        // Process the unconfirmed transaction.
        {
            let transaction_id = transaction.id();

            // Check that the transaction is not a fee transaction.
            if transaction.is_fee() {
                bail!("Transaction '{}' is a fee transaction {}", fmt_id(transaction_id), "(skipping)".dimmed());
            }
            // Check if the transaction was recently seen.
            if self.seen_transactions.lock().put(transaction_id, ()).is_some() {
                // If the transaction was recently seen, return early.
                return Ok(());
            }
            // Check if the transaction is in the transactions_in_queue.
            if self.transactions_in_queue.lock().contains(&transaction_id) {
                // If the transaction is in the queue, return early.
                return Ok(());
            }
            // Check if the transaction already exists in the ledger.
            if self.ledger.contains_transmission(&TransmissionID::from(&transaction_id))? {
                bail!("Transaction '{}' exists in the ledger {}", fmt_id(transaction_id), "(skipping)".dimmed());
            }
            // Check if the queue is full.
            if std::mem::size_of_val(&self.transactions_queue.lock()) >= MAX_TX_QUEUE_SIZE {
                // If the queue is full, return early.
                return Ok(());
            }
            // Get the priority_fee amount from the Deploy or Execute Transaction. Fee Transactions are not expected here.
            let priority_fee = match &transaction {
                Transaction::Deploy(_, _, _, fee) => fee.priority_amount()?,
                Transaction::Execute(_, _, Some(fee)) => fee.priority_amount()?,
                Transaction::Execute(_, _, None) => U64::new(0),
                Transaction::Fee(..) => {
                    bail!("Transaction '{}' is a fee transaction {}", fmt_id(transaction_id), "(skipping)".dimmed());
                }
            };
            // Add the transaction to the memory pool, ordered by priority_fee and oldest timestamp.
            trace!("Received unconfirmed transaction '{}' in the queue", fmt_id(transaction_id));
            if self.add_transaction_to_queue(priority_fee, transaction_id, transaction).is_err() {
                bail!("Transaction '{}' exists in the memory pool", fmt_id(transaction_id));
            }
        }

        // If the memory pool of this node is full, return early.
        let num_unconfirmed = self.num_unconfirmed_transmissions();
        if num_unconfirmed > MAX_TRANSMISSIONS_PER_BATCH {
            return Ok(());
        }
        // Retrieve the transactions.
        let transactions = {
            // Determine the available capacity.
            let capacity = MAX_TRANSMISSIONS_PER_BATCH.saturating_sub(num_unconfirmed);
            // Pop the transactions from the queue.
            self.pop_transactions_from_queue(capacity)
        };
        // Iterate over the transactions.
        for (_, transaction) in transactions.into_iter() {
            let transaction_id = transaction.id();
            trace!("Adding unconfirmed transaction '{}' to the memory pool...", fmt_id(transaction_id));
            // Send the unconfirmed transaction to the primary.
            if let Err(e) =
                self.primary_sender().send_unconfirmed_transaction(transaction_id, Data::Object(transaction)).await
            {
                warn!("Failed to add unconfirmed transaction '{}' to the memory pool - {e}", fmt_id(transaction_id));
            }
        }
        Ok(())
    }

    /// Adds the given unconfirmed transaction to the transaction_queue
    fn add_transaction_to_queue(
        &self,
        priority_fee: PriorityFee<N>,
        id: N::TransactionID,
        transaction: Transaction<N>,
    ) -> Result<()> {
        // Get the current timestamp.
        let timestamp = now_nanos();
        // Add the Transaction to the transactions_in_queue.
        let inserted = self.transactions_in_queue.lock().insert(id);
        ensure!(inserted, "Transaction '{}' exists in the queue", fmt_id(id));
        // Add the Transaction to the queue.
        let found = self.transactions_queue.lock().insert((priority_fee, -timestamp, *id.deref()), transaction);
        ensure!(found.is_none(), "Transaction '{}' exists in the memory pool", fmt_id(id));
        Ok(())
    }

    /// Pops an unconfirmed transaction from the transaction_queue
    fn pop_transactions_from_queue(&self, capacity: usize) -> Vec<(TransactionKey<N>, Transaction<N>)> {
        // Acquire the lock on the queue.
        let mut queue = self.transactions_queue.lock();
        // Determine the number of transactions to send.
        let num_transactions = queue.len().min(capacity);
        // Pop the transactions from the queue.
        let transactions = (0..num_transactions).map(|_| queue.pop_last().unwrap()).collect::<Vec<_>>();
        // Remove the transactions from the queue index and mark them again freshly as seen.
        for (_, tx) in transactions.iter() {
            self.transactions_in_queue.lock().remove(&tx.id());
            self.seen_transactions.lock().put(tx.id(), ());
        }
        transactions
    }
}

impl<N: Network> Consensus<N> {
    /// Starts the consensus handlers.
    fn start_handlers(&self, consensus_receiver: ConsensusReceiver<N>) {
        let ConsensusReceiver { mut rx_consensus_subdag } = consensus_receiver;

        // Process the committed subdag and transmissions from the BFT.
        let self_ = self.clone();
        self.spawn(async move {
            while let Some((committed_subdag, transmissions, callback)) = rx_consensus_subdag.recv().await {
                self_.process_bft_subdag(committed_subdag, transmissions, callback).await;
            }
        });
    }

    /// Processes the committed subdag and transmissions from the BFT.
    async fn process_bft_subdag(
        &self,
        subdag: Subdag<N>,
        transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
        callback: oneshot::Sender<Result<()>>,
    ) {
        // Try to advance to the next block.
        let self_ = self.clone();
        let transmissions_ = transmissions.clone();
        let result = spawn_blocking! { self_.try_advance_to_next_block(subdag, transmissions_) };

        // If the block failed to advance, reinsert the transmissions into the memory pool.
        if let Err(e) = &result {
            error!("Unable to advance to the next block - {e}");
            // On failure, reinsert the transmissions into the memory pool.
            self.reinsert_transmissions(transmissions).await;
        }
        // Send the callback **after** advancing to the next block.
        // Note: We must await the block to be advanced before sending the callback.
        callback.send(result).ok();
    }

    /// Attempts to advance to the next block.
    fn try_advance_to_next_block(
        &self,
        subdag: Subdag<N>,
        transmissions: IndexMap<TransmissionID<N>, Transmission<N>>,
    ) -> Result<()> {
        #[cfg(feature = "metrics")]
        let start = subdag.leader_certificate().batch_header().timestamp();
        #[cfg(feature = "metrics")]
        let num_committed_certificates = subdag.values().map(|c| c.len()).sum::<usize>();
        #[cfg(feature = "metrics")]
        let current_block_timestamp = self.ledger.latest_block().header().metadata().timestamp();

        // Create the candidate next block.
        let next_block = self.ledger.prepare_advance_to_next_quorum_block(subdag, transmissions)?;
        // Check that the block is well-formed.
        self.ledger.check_next_block(&next_block)?;
        // Advance to the next block.
        self.ledger.advance_to_next_block(&next_block)?;

        #[cfg(feature = "metrics")]
        {
            let elapsed = std::time::Duration::from_secs((snarkos_node_bft::helpers::now() - start) as u64);
            let next_block_timestamp = next_block.header().metadata().timestamp();
            let block_latency = next_block_timestamp - current_block_timestamp;

            metrics::gauge(metrics::blocks::HEIGHT, next_block.height() as f64);
            metrics::counter(metrics::blocks::TRANSACTIONS, next_block.transactions().len() as u64);
            metrics::gauge(metrics::consensus::LAST_COMMITTED_ROUND, next_block.round() as f64);
            metrics::gauge(metrics::consensus::COMMITTED_CERTIFICATES, num_committed_certificates as f64);
            metrics::histogram(metrics::consensus::CERTIFICATE_COMMIT_LATENCY, elapsed.as_secs_f64());
            metrics::histogram(metrics::consensus::BLOCK_LATENCY, block_latency as f64);
        }
        Ok(())
    }

    /// Reinserts the given transmissions into the memory pool.
    async fn reinsert_transmissions(&self, transmissions: IndexMap<TransmissionID<N>, Transmission<N>>) {
        // Iterate over the transmissions.
        for (transmission_id, transmission) in transmissions.into_iter() {
            // Reinsert the transmission into the memory pool.
            if let Err(e) = self.reinsert_transmission(transmission_id, transmission).await {
                warn!("Unable to reinsert transmission {} into the memory pool - {e}", fmt_id(transmission_id));
            }
        }
    }

    /// Reinserts the given transmission into the memory pool.
    async fn reinsert_transmission(
        &self,
        transmission_id: TransmissionID<N>,
        transmission: Transmission<N>,
    ) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback, callback_receiver) = oneshot::channel();
        // Send the transmission to the primary.
        match (transmission_id, transmission) {
            (TransmissionID::Ratification, Transmission::Ratification) => return Ok(()),
            (TransmissionID::Solution(commitment), Transmission::Solution(solution)) => {
                // Remove the solution from seen_solutions if it exists.
                self.seen_solutions.lock().pop_entry(&commitment);
                // Send the solution to the primary.
                self.primary_sender().tx_unconfirmed_solution.send((commitment, solution, callback)).await?;
            }
            (TransmissionID::Transaction(transaction_id), Transmission::Transaction(transaction)) => {
                // Remove the transaction from seen_transactions if it exists.
                self.seen_transactions.lock().pop_entry(&transaction_id);
                // Send the transaction to the primary.
                self.primary_sender().tx_unconfirmed_transaction.send((transaction_id, transaction, callback)).await?;
            }
            _ => bail!("Mismatching `(transmission_id, transmission)` pair in consensus"),
        }
        // Await the callback.
        callback_receiver.await?
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the BFT.
    pub async fn shut_down(&self) {
        info!("Shutting down consensus...");
        // Shut down the BFT.
        self.bft.shut_down().await;
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;
    use snarkos_account::Account;
    use snarkos_node_bft_ledger_service::MockLedgerService;
    use snarkvm::{
        console::{
            network::Testnet3,
            types::{Address, U64},
        },
        ledger::{committee::Committee, ledger_test_helpers::sample_deployment_transaction},
        prelude::{Itertools, Uniform},
        utilities::TestRng,
    };
    use std::{ops::Deref, sync::Arc};

    use crate::Consensus;

    type CurrentNetwork = Testnet3;

    #[test]
    fn test_transactions_queue_ordering() {
        let rng = &mut TestRng::default();

        // Sample a committee and mock ledger service.
        let num_validators = 3;
        let minimum_stake = 1000000000000;
        let accepts_delegators = true;
        let members = (0..num_validators)
            .map(|_| (Address::<CurrentNetwork>::rand(rng), (minimum_stake, accepts_delegators)))
            .collect::<IndexMap<_, _>>();
        let committee = Committee::<CurrentNetwork>::new(0, members).unwrap();
        let ledger = Arc::new(MockLedgerService::new(committee));

        // Create Consensus.
        let consensus =
            Consensus::<CurrentNetwork>::new(Account::new(rng).unwrap(), ledger, Default::default(), &[], None)
                .unwrap();

        // Sample priority_fees.
        let fee_amounts = [100_000, 200_000, 300_000, 300_000];
        let priority_fees = fee_amounts.into_iter().map(U64::new).collect_vec();

        // Sample Transactions
        let transactions = (0..4).map(|_| sample_deployment_transaction(false, rng)).collect_vec();

        // Save Transaction ids.
        let transaction_ids = transactions.iter().map(|t| t.id()).collect_vec();

        // Insert Transactions into the queue with sleeps.
        for (tx, priority_fee) in transactions.iter().zip_eq(priority_fees.iter()) {
            assert!(consensus.add_transaction_to_queue(*priority_fee, tx.id(), tx.clone()).is_ok());
            // Sleep for 1 second.
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        // When the fee is the same, the oldest timestamp is popped first.
        // When the fee is different, the highest fee is popped first.
        // So we expect the above transactions to be popped in order 2, 3, 1, 0.
        let expected_ids = [2, 3, 1, 0];
        for expected_id in expected_ids {
            let mut transactions = consensus.pop_transactions_from_queue(1);
            assert_eq!(transactions.len(), 1);
            let (priority_fee, _, id) = transactions.pop().unwrap().0;
            assert_eq!(priority_fee, priority_fees[expected_id]);
            assert_eq!(id, *transaction_ids[expected_id].deref());
        }

        // If the queue is empty, no transactions are popped.
        assert!(consensus.transactions_queue.lock().is_empty());
        assert!(consensus.transactions_in_queue.lock().is_empty());
        let queue_is_empty = consensus.pop_transactions_from_queue(1).is_empty();
        assert!(queue_is_empty);

        // Insert Transactions into the queue without sleeps.
        for (tx, priority_fee) in transactions.iter().zip_eq(priority_fees.iter()) {
            assert!(consensus.add_transaction_to_queue(*priority_fee, tx.id(), tx.clone()).is_ok());
        }
        // We expect all transactions to be in the queue, with unknown order because of similar timestamps.
        let transactions = consensus.pop_transactions_from_queue(4);
        assert_eq!(transactions.len(), 4);
        assert!(consensus.transactions_queue.lock().is_empty());
        assert!(consensus.transactions_in_queue.lock().is_empty());
    }
}
