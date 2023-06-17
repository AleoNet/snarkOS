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

use snarkvm::{
    console::network::*,
    prelude::{ProverSolution, Transaction},
};

use tokio::sync::mpsc;

const GATEWAY_CHANNEL_SIZE: usize = 1024;

/// Initializes the gateway channels.
pub fn init_gateway_channels<N: Network>() -> (GatewaySender<N>, GatewayReceiver<N>) {
    let (tx_unconfirmed_solution, rx_unconfirmed_solution) = mpsc::channel(GATEWAY_CHANNEL_SIZE);
    let (tx_unconfirmed_transaction, rx_unconfirmed_transaction) = mpsc::channel(GATEWAY_CHANNEL_SIZE);

    let gateway_sender = GatewaySender { tx_unconfirmed_solution, tx_unconfirmed_transaction };

    let gateway_receiver = GatewayReceiver { rx_unconfirmed_solution, rx_unconfirmed_transaction };

    (gateway_sender, gateway_receiver)
}

#[derive(Debug)]
pub struct GatewaySender<N: Network> {
    pub tx_unconfirmed_solution: mpsc::Sender<ProverSolution<N>>,
    pub tx_unconfirmed_transaction: mpsc::Sender<Transaction<N>>,
}

#[derive(Debug)]
pub struct GatewayReceiver<N: Network> {
    pub rx_unconfirmed_solution: mpsc::Receiver<ProverSolution<N>>,
    pub rx_unconfirmed_transaction: mpsc::Receiver<Transaction<N>>,
}
