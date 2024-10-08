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

use crate::events::{
    BatchPropose,
    BatchSignature,
    CertificateRequest,
    CertificateResponse,
    TransmissionRequest,
    TransmissionResponse,
};
use snarkos_node_sync::locators::BlockLocators;
use snarkvm::{
    console::network::*,
    ledger::{
        block::{Block, Transaction},
        narwhal::{BatchCertificate, Data, Subdag, Transmission, TransmissionID},
        puzzle::{Solution, SolutionID},
    },
    prelude::Result,
};

use indexmap::IndexMap;
use std::net::SocketAddr;
use tokio::sync::{mpsc, oneshot};

const MAX_CHANNEL_SIZE: usize = 8192;

#[derive(Debug)]
pub struct ConsensusSender<N: Network> {
    pub tx_consensus_subdag:
        mpsc::Sender<(Subdag<N>, IndexMap<TransmissionID<N>, Transmission<N>>, oneshot::Sender<Result<()>>)>,
}

#[derive(Debug)]
pub struct ConsensusReceiver<N: Network> {
    pub rx_consensus_subdag:
        mpsc::Receiver<(Subdag<N>, IndexMap<TransmissionID<N>, Transmission<N>>, oneshot::Sender<Result<()>>)>,
}

/// Initializes the consensus channels.
pub fn init_consensus_channels<N: Network>() -> (ConsensusSender<N>, ConsensusReceiver<N>) {
    let (tx_consensus_subdag, rx_consensus_subdag) = mpsc::channel(MAX_CHANNEL_SIZE);

    let sender = ConsensusSender { tx_consensus_subdag };
    let receiver = ConsensusReceiver { rx_consensus_subdag };

    (sender, receiver)
}

#[derive(Clone, Debug)]
pub struct BFTSender<N: Network> {
    pub tx_primary_round: mpsc::Sender<(u64, oneshot::Sender<bool>)>,
    pub tx_primary_certificate: mpsc::Sender<(BatchCertificate<N>, oneshot::Sender<Result<()>>)>,
    pub tx_sync_bft_dag_at_bootup: mpsc::Sender<Vec<BatchCertificate<N>>>,
    pub tx_sync_bft: mpsc::Sender<(BatchCertificate<N>, oneshot::Sender<Result<()>>)>,
}

impl<N: Network> BFTSender<N> {
    /// Sends the current round to the BFT.
    pub async fn send_primary_round_to_bft(&self, current_round: u64) -> Result<bool> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the current round to the BFT.
        self.tx_primary_round.send((current_round, callback_sender)).await?;
        // Await the callback to continue.
        Ok(callback_receiver.await?)
    }

    /// Sends the batch certificate to the BFT.
    pub async fn send_primary_certificate_to_bft(&self, certificate: BatchCertificate<N>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the certificate to the BFT.
        self.tx_primary_certificate.send((certificate, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }

    /// Sends the batch certificates to the BFT for syncing.
    pub async fn send_sync_bft(&self, certificate: BatchCertificate<N>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the certificate to the BFT for syncing.
        self.tx_sync_bft.send((certificate, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }
}

#[derive(Debug)]
pub struct BFTReceiver<N: Network> {
    pub rx_primary_round: mpsc::Receiver<(u64, oneshot::Sender<bool>)>,
    pub rx_primary_certificate: mpsc::Receiver<(BatchCertificate<N>, oneshot::Sender<Result<()>>)>,
    pub rx_sync_bft_dag_at_bootup: mpsc::Receiver<Vec<BatchCertificate<N>>>,
    pub rx_sync_bft: mpsc::Receiver<(BatchCertificate<N>, oneshot::Sender<Result<()>>)>,
}

/// Initializes the BFT channels.
pub fn init_bft_channels<N: Network>() -> (BFTSender<N>, BFTReceiver<N>) {
    let (tx_primary_round, rx_primary_round) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_primary_certificate, rx_primary_certificate) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_sync_bft_dag_at_bootup, rx_sync_bft_dag_at_bootup) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_sync_bft, rx_sync_bft) = mpsc::channel(MAX_CHANNEL_SIZE);

    let sender = BFTSender { tx_primary_round, tx_primary_certificate, tx_sync_bft_dag_at_bootup, tx_sync_bft };
    let receiver = BFTReceiver { rx_primary_round, rx_primary_certificate, rx_sync_bft_dag_at_bootup, rx_sync_bft };

    (sender, receiver)
}

#[derive(Clone, Debug)]
pub struct PrimarySender<N: Network> {
    pub tx_batch_propose: mpsc::Sender<(SocketAddr, BatchPropose<N>)>,
    pub tx_batch_signature: mpsc::Sender<(SocketAddr, BatchSignature<N>)>,
    pub tx_batch_certified: mpsc::Sender<(SocketAddr, Data<BatchCertificate<N>>)>,
    pub tx_primary_ping: mpsc::Sender<(SocketAddr, Data<BatchCertificate<N>>)>,
    pub tx_unconfirmed_solution: mpsc::Sender<(SolutionID<N>, Data<Solution<N>>, oneshot::Sender<Result<()>>)>,
    pub tx_unconfirmed_transaction: mpsc::Sender<(N::TransactionID, Data<Transaction<N>>, oneshot::Sender<Result<()>>)>,
}

impl<N: Network> PrimarySender<N> {
    /// Sends the unconfirmed solution to the primary.
    pub async fn send_unconfirmed_solution(
        &self,
        solution_id: SolutionID<N>,
        solution: Data<Solution<N>>,
    ) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the unconfirmed solution to the primary.
        self.tx_unconfirmed_solution.send((solution_id, solution, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }

    /// Sends the unconfirmed transaction to the primary.
    pub async fn send_unconfirmed_transaction(
        &self,
        transaction_id: N::TransactionID,
        transaction: Data<Transaction<N>>,
    ) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the unconfirmed transaction to the primary.
        self.tx_unconfirmed_transaction.send((transaction_id, transaction, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }
}

#[derive(Debug)]
pub struct PrimaryReceiver<N: Network> {
    pub rx_batch_propose: mpsc::Receiver<(SocketAddr, BatchPropose<N>)>,
    pub rx_batch_signature: mpsc::Receiver<(SocketAddr, BatchSignature<N>)>,
    pub rx_batch_certified: mpsc::Receiver<(SocketAddr, Data<BatchCertificate<N>>)>,
    pub rx_primary_ping: mpsc::Receiver<(SocketAddr, Data<BatchCertificate<N>>)>,
    pub rx_unconfirmed_solution: mpsc::Receiver<(SolutionID<N>, Data<Solution<N>>, oneshot::Sender<Result<()>>)>,
    pub rx_unconfirmed_transaction:
        mpsc::Receiver<(N::TransactionID, Data<Transaction<N>>, oneshot::Sender<Result<()>>)>,
}

/// Initializes the primary channels.
pub fn init_primary_channels<N: Network>() -> (PrimarySender<N>, PrimaryReceiver<N>) {
    let (tx_batch_propose, rx_batch_propose) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_batch_signature, rx_batch_signature) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_batch_certified, rx_batch_certified) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_primary_ping, rx_primary_ping) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_unconfirmed_solution, rx_unconfirmed_solution) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_unconfirmed_transaction, rx_unconfirmed_transaction) = mpsc::channel(MAX_CHANNEL_SIZE);

    let sender = PrimarySender {
        tx_batch_propose,
        tx_batch_signature,
        tx_batch_certified,
        tx_primary_ping,
        tx_unconfirmed_solution,
        tx_unconfirmed_transaction,
    };
    let receiver = PrimaryReceiver {
        rx_batch_propose,
        rx_batch_signature,
        rx_batch_certified,
        rx_primary_ping,
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

#[derive(Debug)]
pub struct SyncSender<N: Network> {
    pub tx_block_sync_advance_with_sync_blocks: mpsc::Sender<(SocketAddr, Vec<Block<N>>, oneshot::Sender<Result<()>>)>,
    pub tx_block_sync_remove_peer: mpsc::Sender<SocketAddr>,
    pub tx_block_sync_update_peer_locators: mpsc::Sender<(SocketAddr, BlockLocators<N>, oneshot::Sender<Result<()>>)>,
    pub tx_certificate_request: mpsc::Sender<(SocketAddr, CertificateRequest<N>)>,
    pub tx_certificate_response: mpsc::Sender<(SocketAddr, CertificateResponse<N>)>,
}

impl<N: Network> SyncSender<N> {
    /// Sends the request to update the peer locators.
    pub async fn update_peer_locators(&self, peer_ip: SocketAddr, block_locators: BlockLocators<N>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the request to update the peer locators.
        self.tx_block_sync_update_peer_locators.send((peer_ip, block_locators, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }

    /// Sends the request to advance with sync blocks.
    pub async fn advance_with_sync_blocks(&self, peer_ip: SocketAddr, blocks: Vec<Block<N>>) -> Result<()> {
        // Initialize a callback sender and receiver.
        let (callback_sender, callback_receiver) = oneshot::channel();
        // Send the request to advance with sync blocks.
        self.tx_block_sync_advance_with_sync_blocks.send((peer_ip, blocks, callback_sender)).await?;
        // Await the callback to continue.
        callback_receiver.await?
    }
}

#[derive(Debug)]
pub struct SyncReceiver<N: Network> {
    pub rx_block_sync_advance_with_sync_blocks:
        mpsc::Receiver<(SocketAddr, Vec<Block<N>>, oneshot::Sender<Result<()>>)>,
    pub rx_block_sync_remove_peer: mpsc::Receiver<SocketAddr>,
    pub rx_block_sync_update_peer_locators: mpsc::Receiver<(SocketAddr, BlockLocators<N>, oneshot::Sender<Result<()>>)>,
    pub rx_certificate_request: mpsc::Receiver<(SocketAddr, CertificateRequest<N>)>,
    pub rx_certificate_response: mpsc::Receiver<(SocketAddr, CertificateResponse<N>)>,
}

/// Initializes the sync channels.
pub fn init_sync_channels<N: Network>() -> (SyncSender<N>, SyncReceiver<N>) {
    let (tx_block_sync_advance_with_sync_blocks, rx_block_sync_advance_with_sync_blocks) =
        mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_block_sync_remove_peer, rx_block_sync_remove_peer) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_block_sync_update_peer_locators, rx_block_sync_update_peer_locators) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_certificate_request, rx_certificate_request) = mpsc::channel(MAX_CHANNEL_SIZE);
    let (tx_certificate_response, rx_certificate_response) = mpsc::channel(MAX_CHANNEL_SIZE);

    let sender = SyncSender {
        tx_block_sync_advance_with_sync_blocks,
        tx_block_sync_remove_peer,
        tx_block_sync_update_peer_locators,
        tx_certificate_request,
        tx_certificate_response,
    };
    let receiver = SyncReceiver {
        rx_block_sync_advance_with_sync_blocks,
        rx_block_sync_remove_peer,
        rx_block_sync_update_peer_locators,
        rx_certificate_request,
        rx_certificate_response,
    };

    (sender, receiver)
}
