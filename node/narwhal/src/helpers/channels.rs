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

use crate::{EntryRequest, EntryResponse, Ping};
use snarkos_node_messages::Data;
use snarkvm::{
    console::network::*,
    prelude::{ProverSolution, PuzzleCommitment, Transaction},
};

use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;

const GATEWAY_CHANNEL_SIZE: usize = 8192;

/// Initializes the primary channels.
pub fn init_primary_channels<N: Network>() -> (PrimarySender<N>, PrimaryReceiver<N>) {
    let (tx_unconfirmed_solution, rx_unconfirmed_solution) = mpsc::channel(GATEWAY_CHANNEL_SIZE);
    let (tx_unconfirmed_transaction, rx_unconfirmed_transaction) = mpsc::channel(GATEWAY_CHANNEL_SIZE);

    let tx_unconfirmed_solution = Arc::new(tx_unconfirmed_solution);
    let tx_unconfirmed_transaction = Arc::new(tx_unconfirmed_transaction);

    let sender = PrimarySender { tx_unconfirmed_solution, tx_unconfirmed_transaction };
    let receiver = PrimaryReceiver { rx_unconfirmed_solution, rx_unconfirmed_transaction };

    (sender, receiver)
}

#[derive(Debug)]
pub struct PrimarySender<N: Network> {
    pub tx_unconfirmed_solution: Arc<mpsc::Sender<(PuzzleCommitment<N>, Data<ProverSolution<N>>)>>,
    pub tx_unconfirmed_transaction: Arc<mpsc::Sender<(N::TransactionID, Data<Transaction<N>>)>>,
}

#[derive(Debug)]
pub struct PrimaryReceiver<N: Network> {
    pub rx_unconfirmed_solution: mpsc::Receiver<(PuzzleCommitment<N>, Data<ProverSolution<N>>)>,
    pub rx_unconfirmed_transaction: mpsc::Receiver<(N::TransactionID, Data<Transaction<N>>)>,
}

/// Initializes the worker channels.
pub fn init_worker_channels<N: Network>() -> (WorkerSender<N>, WorkerReceiver<N>) {
    let (tx_ping, rx_ping) = mpsc::channel(GATEWAY_CHANNEL_SIZE);
    let (tx_entry_request, rx_entry_request) = mpsc::channel(GATEWAY_CHANNEL_SIZE);
    let (tx_entry_response, rx_entry_response) = mpsc::channel(GATEWAY_CHANNEL_SIZE);

    let tx_ping = Arc::new(tx_ping);
    let tx_entry_request = Arc::new(tx_entry_request);
    let tx_entry_response = Arc::new(tx_entry_response);

    let sender = WorkerSender { tx_ping, tx_entry_request, tx_entry_response };
    let receiver = WorkerReceiver { rx_ping, rx_entry_request, rx_entry_response };

    (sender, receiver)
}

#[derive(Debug)]
pub struct WorkerSender<N: Network> {
    pub tx_ping: Arc<mpsc::Sender<(SocketAddr, Ping<N>)>>,
    pub tx_entry_request: Arc<mpsc::Sender<(SocketAddr, EntryRequest<N>)>>,
    pub tx_entry_response: Arc<mpsc::Sender<(SocketAddr, EntryResponse<N>)>>,
}

#[derive(Debug)]
pub struct WorkerReceiver<N: Network> {
    pub rx_ping: mpsc::Receiver<(SocketAddr, Ping<N>)>,
    pub rx_entry_request: mpsc::Receiver<(SocketAddr, EntryRequest<N>)>,
    pub rx_entry_response: mpsc::Receiver<(SocketAddr, EntryResponse<N>)>,
}
