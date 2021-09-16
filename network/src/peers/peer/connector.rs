// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use std::{
    io::{Error as IoError, ErrorKind},
    time::Duration,
};

use tokio::{net::TcpStream, time::timeout};

use snarkos_metrics::{self as metrics, connections::*, wrapped_mpsc};

use crate::{NetworkError, Node, Peer, PeerEvent, PeerEventData, PeerHandle, Version};

use super::{network::PeerIOHandle, PeerAction};

const CONNECTION_TIMEOUT_SECS: u64 = 3;

impl Peer {
    pub fn connect(mut self, node: Node, event_target: wrapped_mpsc::Sender<PeerEvent>) {
        let (sender, receiver) = wrapped_mpsc::channel::<PeerAction>(snarkos_metrics::queues::PEER_EVENTS, 64);
        tokio::spawn(async move {
            self.set_connecting();
            match self.inner_connect(node.version()).await {
                Err(e) => {
                    self.quality.connect_failed();
                    event_target
                        .send(PeerEvent {
                            address: self.address,
                            data: PeerEventData::FailHandshake,
                        })
                        .await
                        .ok();
                    self.fail();
                    if !e.is_trivial() {
                        error!(
                            "failed to send outgoing connection to peer '{}': '{:?}'",
                            self.address, e
                        );
                    } else {
                        warn!(
                            "failed to send outgoing connection to peer '{}': '{:?}'",
                            self.address, e
                        );
                    }
                }
                Ok(network) => {
                    self.set_connected();
                    metrics::increment_gauge!(CONNECTED, 1.0);
                    event_target
                        .send(PeerEvent {
                            address: self.address,
                            data: PeerEventData::Connected(PeerHandle { sender: sender.clone() }),
                        })
                        .await
                        .ok();

                    if let Err(e) = self.run(node, network, receiver).await {
                        if !e.is_trivial() {
                            self.fail();
                            error!(
                                "unrecoverable failure communicating to outbound peer '{}': '{:?}'",
                                self.address, e
                            );
                        } else {
                            warn!(
                                "unrecoverable failure communicating to outbound peer '{}': '{:?}'",
                                self.address, e
                            );
                        }
                    }
                    metrics::decrement_gauge!(CONNECTED, 1.0);
                }
            }
            self.set_disconnected();
            event_target
                .send(PeerEvent {
                    address: self.address,
                    data: PeerEventData::Disconnect(Box::new(self)),
                })
                .await
                .ok();
        });
    }

    async fn inner_connect(&mut self, our_version: Version) -> Result<PeerIOHandle, NetworkError> {
        metrics::increment_gauge!(CONNECTING, 1.0);
        let _x = defer::defer(|| metrics::decrement_gauge!(CONNECTING, 1.0));

        match timeout(
            Duration::from_secs(CONNECTION_TIMEOUT_SECS),
            TcpStream::connect(self.address),
        )
        .await
        {
            Ok(stream) => self.inner_handshake_initiator(stream?, our_version).await,
            Err(_) => Err(NetworkError::Io(IoError::new(
                ErrorKind::TimedOut,
                "connection timed out",
            ))),
        }
    }
}
