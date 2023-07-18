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

mod memory_pool;
pub use memory_pool::*;

// #[cfg(test)]
// mod tests;

use snarkos_account::Account;
use snarkos_node_narwhal::{
    helpers::{init_consensus_channels, ConsensusReceiver, PrimaryReceiver, PrimarySender, Storage as NarwhalStorage},
    BFT,
    MAX_GC_ROUNDS,
};
use snarkos_node_narwhal_committee::{Committee, MIN_STAKE};
use snarkos_node_narwhal_ledger_service::CoreLedgerService;
use snarkvm::{
    ledger::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        narwhal::{Data, Transmission, TransmissionID},
        store::ConsensusStorage,
    },
    prelude::*,
};

use ::rand::thread_rng;
use anyhow::Result;
use indexmap::IndexMap;
use parking_lot::Mutex;
use std::{future::Future, sync::Arc};
use tokio::{
    sync::{oneshot, OnceCell},
    task::JoinHandle,
};

#[derive(Clone)]
pub struct Consensus<N: Network, C: ConsensusStorage<N>> {
    /// The ledger.
    ledger: Ledger<N, C>,
    /// The BFT.
    bft: BFT<N>,
    /// The primary sender.
    primary_sender: Arc<OnceCell<PrimarySender<N>>>,
    /// The memory pool.
    memory_pool: MemoryPool<N>,
    /// The spawned handles.
    handles: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Initializes a new instance of consensus.
    pub fn new(account: Account<N>, ledger: Ledger<N, C>, dev: Option<u16>) -> Result<Self> {
        // Initialize the committee.
        let committee = {
            // TODO (howardwu): Fix the ledger round number.
            // TODO (howardwu): Retrieve the real committee members.
            // Sample the members.
            let mut members = IndexMap::new();
            for _ in 0..4 {
                members.insert(Address::<N>::new(thread_rng().gen()), MIN_STAKE);
            }
            Committee::new(ledger.latest_round() + 1, members)?
        };
        // Initialize the Narwhal storage.
        let storage = NarwhalStorage::new(committee, MAX_GC_ROUNDS);
        // Initialize the ledger service.
        let ledger_service = Arc::new(CoreLedgerService::<N, C>::new(ledger.clone()));
        // Initialize the BFT.
        let bft = BFT::new(account, storage, ledger_service, None, dev)?;
        // Return the consensus.
        Ok(Self {
            ledger,
            bft,
            primary_sender: Default::default(),
            memory_pool: Default::default(),
            handles: Default::default(),
        })
    }

    /// Run the consensus instance.
    pub async fn run(&mut self, primary_sender: PrimarySender<N>, primary_receiver: PrimaryReceiver<N>) -> Result<()> {
        info!("Starting the consensus instance...");
        // Sets the primary sender.
        self.primary_sender.set(primary_sender.clone()).expect("Primary sender already set");
        // Initialize the consensus channels.
        let (consensus_sender, consensus_receiver) = init_consensus_channels();
        // Start the consensus.
        self.bft.run(primary_sender, primary_receiver, Some(consensus_sender)).await?;
        // Start the consensus handlers.
        self.start_handlers(consensus_receiver);
        Ok(())
    }

    /// Returns the ledger.
    pub const fn ledger(&self) -> &Ledger<N, C> {
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

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
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

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
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

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Adds the given unconfirmed transaction to the memory pool.
    pub async fn add_unconfirmed_transaction(&self, transaction: Transaction<N>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback, callback_receiver) = oneshot::channel();
        // Send the transaction to the primary.
        self.primary_sender()
            .tx_unconfirmed_transaction
            .send((transaction.id(), Data::Object(transaction), callback))
            .await?;
        // Return the callback.
        callback_receiver.await?
    }

    /// Adds the given unconfirmed solution to the memory pool.
    pub async fn add_unconfirmed_solution(&self, solution: ProverSolution<N>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback, callback_receiver) = oneshot::channel();
        // Send the transaction to the primary.
        self.primary_sender()
            .tx_unconfirmed_solution
            .send((solution.commitment(), Data::Object(solution), callback))
            .await?;
        // Return the callback.
        callback_receiver.await?
    }
}

impl<N: Network, C: ConsensusStorage<N>> Consensus<N, C> {
    /// Starts the consensus handlers.
    fn start_handlers(&self, consensus_receiver: ConsensusReceiver<N>) {
        let ConsensusReceiver { mut rx_consensus_subdag } = consensus_receiver;

        // Process the committed subdag and transmissions from the BFT.
        let _self_ = self.clone();
        self.spawn(async move {
            while let Some((_committed_subdag, _transmissions)) = rx_consensus_subdag.recv().await {
                // TODO (howardwu): Prepare to create a new block.
            }
        });
    }

    /// Spawns a task with the given future; it should only be used for long-running tasks.
    fn spawn<T: Future<Output = ()> + Send + 'static>(&self, future: T) {
        self.handles.lock().push(tokio::spawn(future));
    }

    /// Shuts down the BFT.
    pub async fn shut_down(&self) {
        trace!("Shutting down consensus...");
        // Shut down the BFT.
        self.bft.shut_down().await;
        // Abort the tasks.
        self.handles.lock().iter().for_each(|handle| handle.abort());
    }
}
