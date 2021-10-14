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

use anyhow::*;
use chrono::Utc;
use snarkos_metrics::wrapped_mpsc;
use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use super::{PeerQuality, SyncState};
use crate::{message::Payload, BlockCache, NetworkError, Node};

use super::{network::*, outbound_handler::*};
/// A data structure containing information about a peer.
#[derive(Debug, Clone)]
pub struct Peer {
    /// The address of the node's listener socket.
    pub address: SocketAddr,
    /// The latest broadcast block height of the peer.
    pub block_height: u32,
    /// Quantifies the node's connection quality with the peer.
    pub quality: PeerQuality,
    /// Tracks the node's sync state with the peer.
    pub sync_state: SyncState,

    /// The cache of received blocks from the peer.
    pub block_received_cache: BlockCache<{ crate::PEER_BLOCK_CACHE_SIZE }>,
}

const FAILURE_EXPIRY_TIME: Duration = Duration::from_secs(15 * 60);
const FAILURE_THRESHOLD: usize = 5;

impl Peer {
    pub fn new(address: SocketAddr, data: Option<&snarkos_storage::Peer>) -> Self {
        let mut block_height = 0;
        let mut quality: PeerQuality = Default::default();
        let mut sync_state: SyncState = Default::default();

        if let Some(data) = data {
            block_height = data.block_height;
            quality.sync_from_storage(data);
            sync_state.sync_from_storage(data);
        }

        Self {
            address,
            block_height,
            quality,
            sync_state,

            block_received_cache: BlockCache::default(),
        }
    }

    pub fn serialize(&self) -> snarkos_storage::Peer {
        snarkos_storage::Peer {
            address: self.address,
            block_height: self.block_height,
            first_seen: self.quality.first_seen,
            last_seen: self.quality.last_seen,
            last_connected: self.quality.last_connected,
            blocks_synced_to: self.sync_state.blocks_synced_to,
            blocks_synced_from: self.sync_state.blocks_synced_from,
            blocks_received_from: self.sync_state.blocks_received_from,
            blocks_sent_to: self.sync_state.blocks_sent_to,
            connection_attempt_count: self.quality.connection_attempt_count,
            connection_success_count: self.quality.connected_count,
            connection_transient_fail_count: self.quality.connection_transient_fail_count,
        }
    }

    pub fn judge_bad(&mut self) -> bool {
        let f = self.failures();
        // self.quality.rtt_ms > 1500 ||
        f >= FAILURE_THRESHOLD || self.quality.is_inactive(chrono::Utc::now())
    }

    pub fn judge_bad_offline(&mut self) -> bool {
        self.failures() >= FAILURE_THRESHOLD
    }

    pub fn fail(&mut self) {
        self.quality.failures.push(Utc::now());
    }

    pub fn failures(&mut self) -> usize {
        let now = Utc::now();
        if self.quality.failures.len() >= FAILURE_THRESHOLD {
            self.quality.failures = self
                .quality
                .failures
                .iter()
                .filter(|x| now.signed_duration_since(**x) < chrono::Duration::from_std(FAILURE_EXPIRY_TIME).unwrap())
                .copied()
                .collect();
        }
        self.quality.failures.len()
    }

    pub(super) async fn run(
        &mut self,
        node: Node,
        mut network: PeerIOHandle,
        mut receiver: wrapped_mpsc::Receiver<PeerAction>,
    ) -> Result<(), NetworkError> {
        let mut reader = network.take_reader();

        let (sender, mut read_receiver) =
            wrapped_mpsc::channel::<Result<Vec<u8>, NetworkError>>(snarkos_metrics::queues::INBOUND, 8);

        tokio::spawn(async move {
            loop {
                if sender
                    .send(reader.read_raw_payload().await.map(|x| x.to_vec()))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        loop {
            tokio::select! {
                biased;

                message = receiver.recv() => {
                    if message.is_none() {
                        break;
                    }
                    let message = message.unwrap();
                    match self.process_message(&mut network, message).await? {
                        PeerResponse::Disconnect => break,
                        PeerResponse::None => (),
                    }
                },
                data = read_receiver.recv() => {
                    if data.is_none() {
                        break;
                    }

                    let data = match data.unwrap() {
                        // decrypt
                        Ok(data) => network.read_payload(&data[..]),
                        Err(e) => Err(e)
                    };

                    let deserialized = self.deserialize_payload(data);

                    let time_received = match deserialized {
                        Ok(Payload::GetPeers)
                        | Ok(Payload::GetSync(_))
                        | Ok(Payload::GetBlocks(_))
                        | Ok(Payload::GetMemoryPool) => Some(Instant::now()),
                        _ => None,
                    };

                    self.dispatch_payload(&node, &mut network, time_received, deserialized).await?;
                },
            }
        }

        Ok(())
    }

    pub(super) fn set_connected(&mut self) {
        self.quality.connected();
    }

    pub(super) fn set_connecting(&mut self) {
        self.quality.see();
        self.quality.connecting();
    }

    pub(super) fn set_disconnected(&mut self) {
        self.sync_state.reset();
        self.quality.disconnected();
    }

    pub fn start_ping_measurement(&mut self) {
        self.quality.expecting_pong = true;
        self.quality.last_ping_sent = Some(Instant::now());
    }

    pub fn stop_ping_measurement(&mut self) {
        if !self.quality.expecting_pong {
            self.fail();

            return;
        }

        let rtt = self
            .quality
            .last_ping_sent
            .map(|x| x.elapsed().as_millis() as u64)
            .unwrap_or(u64::MAX);

        trace!("RTT for {} is {}ms", self.address, rtt);

        self.quality.expecting_pong = false;
        self.quality.rtt_ms = rtt;
    }

    pub fn register_received_message(&mut self) {
        self.quality.see();
        self.quality.num_messages_received += 1;
    }

    pub fn register_received_sync_block(&mut self) {
        let remaining_sync_blocks = self.sync_state.remaining_sync_blocks;

        if remaining_sync_blocks == 0 {
            trace!("received unexpected or late sync block from {}", self.address);

            return;
        }

        // Decrement if > 0.
        // Safe since the value can never be negative and would only overflow if equal to 0.
        self.sync_state.remaining_sync_blocks -= 1;
    }

    pub fn increment_sync_expectations(&mut self, amount: u32) {
        self.sync_state.remaining_sync_blocks += amount;
        self.sync_state.total_sync_blocks += amount;
    }

    pub fn cancel_sync(&mut self) {
        if self.sync_state.remaining_sync_blocks == 0 {
            return;
        }

        let message = format!(
            "Was expecting {} more sync blocks from {}",
            self.sync_state.remaining_sync_blocks, self.address
        );

        // Unfortunately the tracing crate doesn't support dynamic log levels, so the `else`
        // statement is necessary here.
        if self.sync_state.remaining_sync_blocks > self.sync_state.total_sync_blocks / 2 {
            warn!("{}", message);
            self.fail();
        } else {
            trace!("{}", message);
        }

        self.sync_state.reset();
    }
}
