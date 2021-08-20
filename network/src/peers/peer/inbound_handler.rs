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

use snarkos_metrics::{
    self as metrics,
    inbound::{self, *},
    misc,
    outbound,
};

use crate::{Direction, KnownNetworkMessage, Message, NetworkError, Node, Payload, Peer, State};

use super::network::PeerIOHandle;

impl Peer {
    pub(super) async fn inner_dispatch_payload(
        &mut self,
        node: &Node,
        network: &mut PeerIOHandle,
        payload: Result<Payload, NetworkError>,
    ) -> Result<(), NetworkError> {
        let payload = payload?;
        self.quality.see();
        self.quality.num_messages_received += 1;

        // If message is a `SyncBlock` message, log it as a trace.
        match payload {
            Payload::SyncBlock(..) => trace!("Received a '{}' message from {}", payload, self.address),
            _ => debug!("Received a '{}' message from {}", payload, self.address),
        }

        match payload {
            Payload::Pong => {
                if self.quality.expecting_pong {
                    let rtt = self
                        .quality
                        .last_ping_sent
                        .map(|x| x.elapsed().as_millis() as u64)
                        .unwrap_or(u64::MAX);
                    trace!("RTT for {} is {}ms", self.address, rtt);
                    self.quality.expecting_pong = false;
                    self.quality.rtt_ms = rtt;
                } else {
                    self.fail();
                }
                metrics::increment_counter!(PONGS);
                metrics::increment_counter!(inbound::ALL_SUCCESSES);
            }
            Payload::Ping(block_height) => {
                network.write_payload(&Payload::Pong).await?;
                debug!("Sent a '{}' message to {}", Payload::Pong, self.address);
                self.quality.block_height = block_height;
                metrics::increment_counter!(PINGS);
                metrics::increment_counter!(inbound::ALL_SUCCESSES);

                // Pongs are sent without going through the outbound handler,
                // so the outbound metric needs to be incremented here
                metrics::increment_counter!(outbound::ALL_SUCCESSES);

                // Relay the height to the known network.
                if let Some(known_network) = node.known_network() {
                    let _ = known_network
                        .sender
                        .try_send(KnownNetworkMessage::Height(self.address, block_height));
                }
            }
            payload => {
                // Check if the message hasn't already been processed recently if it's a `Block`.
                // The node should also reject them while syncing, as it is bound to receive them later.
                if matches!(payload, Payload::Block(..)) {
                    metrics::increment_counter!(inbound::BLOCKS);

                    if node.state() == State::Syncing {
                        return Ok(());
                    }

                    if node.inbound_cache.contains(&payload).await {
                        metrics::increment_counter!(misc::DUPLICATE_BLOCKS);
                        return Ok(());
                    }
                }

                node.route(Message {
                    direction: Direction::Inbound(self.address),
                    payload,
                });
            }
        }

        Ok(())
    }

    pub(super) async fn dispatch_payload(
        &mut self,
        node: &Node,
        network: &mut PeerIOHandle,
        payload: Result<Payload, NetworkError>,
    ) -> Result<(), NetworkError> {
        match self.inner_dispatch_payload(node, network, payload).await {
            Ok(()) => (),
            Err(e) => {
                if e.is_trivial() {
                    trace!("Unable to read message from {}: {}", self.address, e);
                } else {
                    warn!("Unable to read message from {}: {}", self.address, e);
                }
                return Err(e);
            }
        }
        Ok(())
    }

    pub(super) fn deserialize_payload(&self, payload: Result<&[u8], NetworkError>) -> Result<Payload, NetworkError> {
        let payload = payload?;
        let payload = Payload::deserialize(payload)?;
        Ok(payload)
    }
}
