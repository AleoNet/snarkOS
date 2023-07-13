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
    ledger::narwhal::{BatchCertificate, Data, Transmission, TransmissionID},
    prelude::{
        block::Transaction,
        coinbase::{ProverSolution, PuzzleCommitment},
        Result,
    },
};

use indexmap::IndexMap;
use std::{collections::BTreeMap, net::SocketAddr};
use tokio::sync::{mpsc, oneshot};

const MAX_CHANNEL_SIZE: usize = 8192;

#[derive(Debug)]
pub struct ConsensusSender<N: Network> {
    pub tx_consensus_subdag:
        mpsc::Sender<(BTreeMap<u64, Vec<BatchCertificate<N>>>, IndexMap<TransmissionID<N>, Transmission<N>>)>,
}

#[derive(Debug)]
pub struct ConsensusReceiver<N: Network> {
    pub rx_consensus_subdag:
        mpsc::Receiver<(BTreeMap<u64, Vec<BatchCertificate<N>>>, IndexMap<TransmissionID<N>, Transmission<N>>)>,
}

/// Initializes the consensus channels.
pub fn init_consensus_channels<N: Network>() -> (ConsensusSender<N>, ConsensusReceiver<N>) {
    let (tx_consensus_subdag, rx_consensus_subdag) = mpsc::channel(MAX_CHANNEL_SIZE);

    let sender = ConsensusSender { tx_consensus_subdag };
    let receiver = ConsensusReceiver { rx_consensus_subdag };

    (sender, receiver)
}

#[derive(Debug)]
pub struct BFTSender<N: Network> {
    pub tx_primary_round: mpsc::Sender<(u64, oneshot::Sender<Result<()>>)>,
    pub tx_primary_certificate: mpsc::Sender<(BatchCertificate<N>, oneshot::Sender<Result<()>>)>,
}

#[derive(Debug)]
pub struct BFTReceiver<N: Network> {
    pub rx_primary_round: mpsc::Receiver<(u64, oneshot::Sender<Result<()>>)>,
    pub rx_primary_certificate: mpsc::Receiver<(BatchCertificate<N>, oneshot::Sender<Result<()>>)>,
}

/// Initializes the BFT channels.
pub fn init_bft_channels<N: Network>() -> (BFTSender<N>, BFTReceiver<N>) {
    let (tx_primary_round, rx_primary_round) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_primary_certificate, rx_primary_certificate) = mpsc::channel(MAX_CHANNEL_SIZE);

    let sender = BFTSender { tx_primary_round, tx_primary_certificate };
    let receiver = BFTReceiver { rx_primary_round, rx_primary_certificate };

    (sender, receiver)
}

#[derive(Clone, Debug)]
pub struct PrimarySender<N: Network> {
    pub tx_batch_propose: mpsc::Sender<(SocketAddr, BatchPropose<N>)>,
    pub tx_batch_signature: mpsc::Sender<(SocketAddr, BatchSignature<N>)>,
    pub tx_batch_certified: mpsc::Sender<(SocketAddr, Data<BatchCertificate<N>>)>,
    pub tx_certificate_request: mpsc::Sender<(SocketAddr, CertificateRequest<N>)>,
    pub tx_certificate_response: mpsc::Sender<(SocketAddr, CertificateResponse<N>)>,
    pub tx_unconfirmed_solution: mpsc::Sender<(PuzzleCommitment<N>, Data<ProverSolution<N>>)>,
    pub tx_unconfirmed_transaction: mpsc::Sender<(N::TransactionID, Data<Transaction<N>>)>,
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
    pub tx_worker_ping: mpsc::Sender<(SocketAddr, TransmissionID<N>)>,
    pub tx_transmission_request: mpsc::Sender<(SocketAddr, TransmissionRequest<N>)>,
    pub tx_transmission_response: mpsc::Sender<(SocketAddr, TransmissionResponse<N>)>,
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

    let sender = WorkerSender { tx_worker_ping, tx_transmission_request, tx_transmission_response };
    let receiver = WorkerReceiver { rx_worker_ping, rx_transmission_request, rx_transmission_response };

    (sender, receiver)
}
