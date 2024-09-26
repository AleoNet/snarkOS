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

use crate::common::{
    primary::new_test_committee,
    utils::{sample_ledger, sample_worker},
    CurrentNetwork,
};
use snarkos_node_bft::helpers::max_redundant_requests;
use snarkvm::{
    ledger::{committee::Committee, narwhal::TransmissionID},
    prelude::{Network, TestRng},
};

use std::net::SocketAddr;

#[tokio::test]
#[rustfmt::skip]
async fn test_resend_transmission_request() {
    const NUM_NODES: u16 = Committee::<CurrentNetwork>::MAX_COMMITTEE_SIZE;

    // Initialize the RNG.
    let mut rng = TestRng::default();
    // Initialize the accounts and the committee.
    let (accounts, committee) = new_test_committee(NUM_NODES, &mut rng);
    // Sample a ledger.
    let ledger = sample_ledger(&accounts, &committee, &mut rng);
    // Sample a worker.
    let worker = sample_worker(0, accounts[0].clone(), ledger.clone());

    // Determine the maximum number of redundant requests.
    let max_redundancy = max_redundant_requests(ledger.clone(), 0);
    assert_eq!(max_redundancy, 34, "Update me if the formula changes");

    // Prepare peer ips.
    let num_test_requests = 11;
    let mut peer_ips = (0..num_test_requests).map(|i| SocketAddr::from(([127, 0, 0, 1], 1234 + i as u16))).collect::<Vec<_>>();
    let initial_peer_ip = peer_ips.pop().unwrap();

    // Prepare a dummy transmission ID.
    let transmission_id = TransmissionID::Transaction(<CurrentNetwork as Network>::TransactionID::default(), <CurrentNetwork as Network>::TransmissionChecksum::default());

    // Ensure the worker does not have the dummy transmission ID.
    assert!(!worker.contains_transmission(transmission_id), "Transmission should not exist");

    // Send a request to fetch the dummy transmission.
    let worker_ = worker.clone();
    tokio::spawn(async move { worker_.get_or_fetch_transmission(initial_peer_ip, transmission_id).await });

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let pending = worker.pending();
    // Ensure the transmission ID exists in the pending queue.
    assert!(pending.contains(transmission_id), "Missing a transmission in the pending queue");
    // Ensure the peer IP is in the pending queue for the transmission ID.
    assert!(pending.contains_peer(transmission_id, initial_peer_ip), "Missing a peer IP for transmission in the pending queue");
    assert_eq!(pending.get_peers(transmission_id), Some([initial_peer_ip].into_iter().collect()), "Missing a peer IP for transmission in the pending queue");
    // Ensure the number of callbacks is correct.
    assert_eq!(pending.num_callbacks(transmission_id), 1, "Incorrect number of callbacks for transmission");
    // Ensure the number of sent requests is correct.
    assert_eq!(pending.num_sent_requests(transmission_id), 1, "Incorrect number of sent requests for transmission");

    // Rebroadcast the same request to the same peer to fetch the dummy transmission.
    for i in 1..num_test_requests {
        let worker_ = worker.clone();
        let peer_ip = initial_peer_ip;
        tokio::spawn(async move { worker_.get_or_fetch_transmission(peer_ip, transmission_id).await });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Ensure the transmission ID exists in the pending queue.
        assert!(pending.contains(transmission_id), "Missing a transmission in the pending queue");
        // Ensure the peer IP is in the pending queue for the transmission ID.
        assert!(pending.contains_peer(transmission_id, peer_ip), "Missing a peer IP for transmission in the pending queue");
        assert_eq!(pending.get_peers(transmission_id), Some([peer_ip].into_iter().collect()), "Missing a peer IP for transmission in the pending queue");
        // Ensure the number of callbacks is correct.
        assert_eq!(pending.num_callbacks(transmission_id), 1 + i, "Incorrect number of callbacks for transmission");
        // Ensure the number of sent requests is correct.
        assert_eq!(pending.num_sent_requests(transmission_id), 1, "Incorrect number of sent requests for transmission");
    }

    // Rebroadcast the same request to new peers fetch the dummy transmission.
    for i in 1..num_test_requests {
        let peer_ip = peer_ips.pop().unwrap();
        let worker_ = worker.clone();
        tokio::spawn(async move { worker_.get_or_fetch_transmission(peer_ip, transmission_id).await });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Ensure the transmission ID exists in the pending queue.
        assert!(pending.contains(transmission_id), "Missing a transmission in the pending queue");
        // Ensure the peer IP is in the pending queue for the transmission ID.
        assert!(pending.contains_peer(transmission_id, peer_ip), "Missing a peer IP for transmission in the pending queue");
        // Ensure the number of sent requests is correct.
        assert_eq!(pending.num_sent_requests(transmission_id), (1 + i).min(max_redundancy), "Incorrect number of sent requests for transmission");
    }
}

#[tokio::test]
#[rustfmt::skip]
async fn test_flood_transmission_requests() {
    const NUM_NODES: u16 = Committee::<CurrentNetwork>::MAX_COMMITTEE_SIZE;

    // Initialize the RNG.
    let mut rng = TestRng::default();
    // Initialize the accounts and the committee.
    let (accounts, committee) = new_test_committee(NUM_NODES, &mut rng);
    // Sample a ledger.
    let ledger = sample_ledger(&accounts, &committee, &mut rng);
    // Sample a worker.
    let worker = sample_worker(0, accounts[0].clone(), ledger.clone());

    // Determine the maximum number of redundant requests.
    let max_redundancy = max_redundant_requests(ledger.clone(), 0);
    assert_eq!(max_redundancy, 34, "Update me if the formula changes");

    // Prepare peer ips.
    let mut peer_ips = (0..max_redundancy + 1).map(|i| SocketAddr::from(([127, 0, 0, 1], 1234 + i as u16))).collect::<Vec<_>>();
    let all_peer_ips = peer_ips.clone();
    let initial_peer_ip = peer_ips.pop().unwrap();
    let mut remaining_peer_ips = peer_ips;

    // Prepare a dummy transmission ID.
    let transmission_id = TransmissionID::Transaction(<CurrentNetwork as Network>::TransactionID::default(), <CurrentNetwork as Network>::TransmissionChecksum::default());

    // Ensure the worker does not have the dummy transmission ID.
    assert!(!worker.contains_transmission(transmission_id), "Transmission should not exist");

    // Send the maximum number of redundant requests to fetch the dummy transmission.
    for peer_ip in remaining_peer_ips.clone() {
        let worker_ = worker.clone();
        tokio::spawn(async move { worker_.get_or_fetch_transmission(peer_ip, transmission_id).await });
    }

    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let pending = worker.pending();
    // Ensure the transmission ID exists in the pending queue.
    assert!(pending.contains(transmission_id), "Missing a transmission in the pending queue");
    // Ensure the peer IP is in the pending queue for the transmission ID.
    assert_eq!(pending.get_peers(transmission_id), Some(remaining_peer_ips.clone().into_iter().collect()), "Missing a peer IP for transmission in the pending queue");
    // Ensure the number of callbacks is correct.
    assert_eq!(pending.num_callbacks(transmission_id), max_redundancy, "Incorrect number of callbacks for transmission");
    // Ensure the number of sent requests is correct.
    assert_eq!(pending.num_sent_requests(transmission_id), max_redundancy, "Incorrect number of sent requests for transmission");

    // Ensure any further redundant requests are not sent when sending to the same peer.
    for i in 1..=20 {
        let worker_ = worker.clone();
        let peer_ip = initial_peer_ip;
        tokio::spawn(async move { worker_.get_or_fetch_transmission(peer_ip, transmission_id).await });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Ensure the transmission ID exists in the pending queue.
        assert!(pending.contains(transmission_id), "Missing a transmission in the pending queue");
        // Ensure the peer IP is in the pending queue for the transmission ID.
        assert!(pending.contains_peer(transmission_id, peer_ip), "Missing a peer IP for transmission in the pending queue");
        assert_eq!(pending.get_peers(transmission_id), Some(all_peer_ips.clone().into_iter().collect()), "Missing a peer IP for transmission in the pending queue");
        // Ensure the number of callbacks is correct.
        assert_eq!(pending.num_callbacks(transmission_id), max_redundancy + i, "Incorrect number of callbacks for transmission");
        // Ensure the number of sent requests is correct.
        assert_eq!(pending.num_sent_requests(transmission_id), max_redundancy, "Incorrect number of sent requests for transmission");
    }

    // Ensure any further redundant requests are not sent when sending to new peers.
    for i in 1..=20 {
        let worker_ = worker.clone();
        let peer_ip = remaining_peer_ips.pop().unwrap();
        tokio::spawn(async move { worker_.get_or_fetch_transmission(peer_ip, transmission_id).await });

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        // Ensure the transmission ID exists in the pending queue.
        assert!(pending.contains(transmission_id), "Missing a transmission in the pending queue");
        // Ensure the peer IP is in the pending queue for the transmission ID.
        assert!(pending.contains_peer(transmission_id, peer_ip), "Missing a peer IP for transmission in the pending queue");
        assert_eq!(pending.get_peers(transmission_id), Some(all_peer_ips.clone().into_iter().collect()), "Missing a peer IP for transmission in the pending queue");
        // Ensure the number of callbacks is correct.
        assert_eq!(pending.num_callbacks(transmission_id), max_redundancy + 20 + i, "Incorrect number of callbacks for transmission");
        // Ensure the number of sent requests is correct.
        assert_eq!(pending.num_sent_requests(transmission_id), max_redundancy, "Incorrect number of sent requests for transmission");
    }
}
