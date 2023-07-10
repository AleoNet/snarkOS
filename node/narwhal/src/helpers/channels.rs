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

use crate::{
    BatchPropose,
    BatchSignature,
    CertificateRequest,
    CertificateResponse,
    TransmissionRequest,
    TransmissionResponse,
};
use snarkvm::{
    console::network::*,
    ledger::narwhal::{BatchCertificate, Data, TransmissionID},
    prelude::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
    },
};

use std::{net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;

const MAX_CHANNEL_SIZE: usize = 8192;

#[derive(Debug)]
pub struct BFTSender<N: Network> {
    pub tx_primary_round: Arc<mpsc::Sender<u64>>,
    _phantom: std::marker::PhantomData<N>,
}

#[derive(Debug)]
pub struct BFTReceiver<N: Network> {
    pub rx_primary_round: mpsc::Receiver<u64>,
    _phantom: std::marker::PhantomData<N>,
}

/// Initializes the BFT channels.
pub fn init_bft_channels<N: Network>() -> (BFTSender<N>, BFTReceiver<N>) {
    let (tx_primary_round, rx_primary_round) = mpsc::channel(MAX_CHANNEL_SIZE);

    let tx_primary_round = Arc::new(tx_primary_round);

    let sender = BFTSender { tx_primary_round, _phantom: std::marker::PhantomData };
    let receiver = BFTReceiver { rx_primary_round, _phantom: std::marker::PhantomData };

    (sender, receiver)
}

#[derive(Clone, Debug)]
pub struct PrimarySender<N: Network> {
    pub tx_batch_propose: Arc<mpsc::Sender<(SocketAddr, BatchPropose<N>)>>,
    pub tx_batch_signature: Arc<mpsc::Sender<(SocketAddr, BatchSignature<N>)>>,
    pub tx_batch_certified: Arc<mpsc::Sender<(SocketAddr, Data<BatchCertificate<N>>)>>,
    pub tx_certificate_request: Arc<mpsc::Sender<(SocketAddr, CertificateRequest<N>)>>,
    pub tx_certificate_response: Arc<mpsc::Sender<(SocketAddr, CertificateResponse<N>)>>,
    pub tx_unconfirmed_solution: Arc<mpsc::Sender<(PuzzleCommitment<N>, Data<ProverSolution<N>>)>>,
    pub tx_unconfirmed_transaction: Arc<mpsc::Sender<(N::TransactionID, Data<Transaction<N>>)>>,
}

#[derive(Debug)]
pub struct PrimaryReceiver<N: Network> {
    pub rx_batch_propose: mpsc::Receiver<(SocketAddr, BatchPropose<N>)>,
    pub rx_batch_signature: mpsc::Receiver<(SocketAddr, BatchSignature<N>)>,
    pub rx_batch_certified: mpsc::Receiver<(SocketAddr, Data<BatchCertificate<N>>)>,
    pub rx_certificate_request: mpsc::Receiver<(SocketAddr, CertificateRequest<N>)>,
    pub rx_certificate_response: mpsc::Receiver<(SocketAddr, CertificateResponse<N>)>,
    pub rx_unconfirmed_solution: mpsc::Receiver<(PuzzleCommitment<N>, Data<ProverSolution<N>>)>,
    pub rx_unconfirmed_transaction: mpsc::Receiver<(N::TransactionID, Data<Transaction<N>>)>,
}

/// Initializes the primary channels.
pub fn init_primary_channels<N: Network>() -> (PrimarySender<N>, PrimaryReceiver<N>) {
    let (tx_batch_propose, rx_batch_propose) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_batch_signature, rx_batch_signature) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_batch_certified, rx_batch_certified) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_certificate_request, rx_certificate_request) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_certificate_response, rx_certificate_response) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_unconfirmed_solution, rx_unconfirmed_solution) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_unconfirmed_transaction, rx_unconfirmed_transaction) = mpsc::channel(MAX_CHANNEL_SIZE);

    let tx_batch_propose = Arc::new(tx_batch_propose);
    let tx_batch_signature = Arc::new(tx_batch_signature);
    let tx_batch_certified = Arc::new(tx_batch_certified);
    let tx_certificate_request = Arc::new(tx_certificate_request);
    let tx_certificate_response = Arc::new(tx_certificate_response);
    let tx_unconfirmed_solution = Arc::new(tx_unconfirmed_solution);
    let tx_unconfirmed_transaction = Arc::new(tx_unconfirmed_transaction);

    let sender = PrimarySender {
        tx_batch_propose,
        tx_batch_signature,
        tx_batch_certified,
        tx_certificate_request,
        tx_certificate_response,
        tx_unconfirmed_solution,
        tx_unconfirmed_transaction,
    };
    let receiver = PrimaryReceiver {
        rx_batch_propose,
        rx_batch_signature,
        rx_batch_certified,
        rx_certificate_request,
        rx_certificate_response,
        rx_unconfirmed_solution,
        rx_unconfirmed_transaction,
    };

    (sender, receiver)
}

#[derive(Debug)]
pub struct WorkerSender<N: Network> {
    pub tx_worker_ping: Arc<mpsc::Sender<(SocketAddr, TransmissionID<N>)>>,
    pub tx_transmission_request: Arc<mpsc::Sender<(SocketAddr, TransmissionRequest<N>)>>,
    pub tx_transmission_response: Arc<mpsc::Sender<(SocketAddr, TransmissionResponse<N>)>>,
}

#[derive(Debug)]
pub struct WorkerReceiver<N: Network> {
    pub rx_worker_ping: mpsc::Receiver<(SocketAddr, TransmissionID<N>)>,
    pub rx_transmission_request: mpsc::Receiver<(SocketAddr, TransmissionRequest<N>)>,
    pub rx_transmission_response: mpsc::Receiver<(SocketAddr, TransmissionResponse<N>)>,
}

/// Initializes the worker channels.
pub fn init_worker_channels<N: Network>() -> (WorkerSender<N>, WorkerReceiver<N>) {
    let (tx_worker_ping, rx_worker_ping) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_transmission_request, rx_transmission_request) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_transmission_response, rx_transmission_response) = mpsc::channel(MAX_CHANNEL_SIZE);

    let tx_worker_ping = Arc::new(tx_worker_ping);
    let tx_transmission_request = Arc::new(tx_transmission_request);
    let tx_transmission_response = Arc::new(tx_transmission_response);

    let sender = WorkerSender { tx_worker_ping, tx_transmission_request, tx_transmission_response };
    let receiver = WorkerReceiver { rx_worker_ping, rx_transmission_request, rx_transmission_response };

    (sender, receiver)
}
