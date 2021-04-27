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

use crate::*;
use snarkvm_objects::Storage;

use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
use rand::{thread_rng, Rng};
use std::{
    collections::HashMap,
    net::SocketAddr,
    ops::Deref,
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread,
};
use tokio::{task, time::sleep};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u8)]
pub enum State {
    Idle = 0,
    Mining,
    Syncing,
}

#[derive(Default)]
pub struct StateCode(AtomicU8);

#[doc(hide)]
pub struct InnerNode<S: Storage> {
    /// The node's random numeric identifier.
    pub name: u64,
    /// The current state of the node.
    state: StateCode,
    /// The pre-configured parameters of this node.
    pub config: Config,
    /// The inbound handler of this node.
    pub inbound: Inbound,
    /// The outbound handler of this node.
    pub outbound: Outbound,
    /// The list of connected and disconnected peers of this node.
    pub peer_book: PeerBook,
    /// The objects related to consensus.
    pub consensus: OnceCell<Arc<Consensus<S>>>,
    /// Node's local address.
    pub local_address: OnceCell<SocketAddr>,
    /// The tasks spawned by the node.
    tasks: Mutex<Vec<task::JoinHandle<()>>>,
    /// The threads spawned by the node.
    threads: Mutex<Vec<thread::JoinHandle<()>>>,
    /// An indicator of whether the node is shutting down.
    shutting_down: AtomicBool,
}

impl<S: Storage> Drop for InnerNode<S> {
    // this won't make a difference in regular scenarios, but will be practical for test
    // purposes, so that there are no lingering tasks
    fn drop(&mut self) {
        // since we're going out of scope, we don't care about holding the read lock here
        // also, the connections are going to be broken automatically, so we only need to
        // take care of the associated tasks here
        for peer_info in self.peer_book.connected_peers().values() {
            for handle in peer_info.tasks.lock().drain(..).rev() {
                handle.abort();
            }
        }

        for handle in self.threads.lock().drain(..).rev() {
            let _ = handle.join().map_err(|e| error!("Can't join a thread: {:?}", e));
        }

        for handle in self.tasks.lock().drain(..).rev() {
            handle.abort();
        }
    }
}

/// A core data structure for operating the networking stack of this node.
#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Node<S: Storage>(Arc<InnerNode<S>>);

impl<S: Storage> Deref for Node<S> {
    type Target = Arc<InnerNode<S>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S: Storage> Node<S> {
    /// Returns the current state of the node.
    #[inline]
    pub fn state(&self) -> State {
        match self.state.0.load(Ordering::SeqCst) {
            0 => State::Idle,
            1 => State::Mining,
            2 => State::Syncing,
            _ => unreachable!(),
        }
    }

    /// Changes the current state of the node.
    #[inline]
    pub fn set_state(&self, new_state: State) {
        let code = new_state as u8;

        self.state.0.store(code, Ordering::SeqCst);
    }
}

impl<S: Storage + Send + Sync + 'static> Node<S> {
    /// Creates a new instance of `Node`.
    pub async fn new(config: Config) -> Result<Self, NetworkError> {
        // Create the inbound and outbound handlers.
        let (inbound, outbound) = {
            let channels: Arc<RwLock<HashMap<SocketAddr, Arc<ConnWriter>>>> = Default::default();
            (Inbound::new(channels.clone()), Outbound::new(channels))
        };

        Ok(Self(Arc::new(InnerNode {
            name: thread_rng().gen(),
            state: Default::default(),
            local_address: Default::default(),
            config,
            inbound,
            outbound,
            peer_book: Default::default(),
            consensus: Default::default(),
            tasks: Default::default(),
            threads: Default::default(),
            shutting_down: Default::default(),
        })))
    }

    pub fn set_consensus(&mut self, consensus: Consensus<S>) {
        if self.consensus.set(Arc::new(consensus)).is_err() {
            panic!("consensus was set more than once!");
        }
    }

    /// Returns a reference to the consensus objects.
    #[inline]
    pub fn consensus(&self) -> Option<&Arc<Consensus<S>>> {
        self.consensus.get()
    }

    /// Returns a reference to the consensus objects, expecting them to be available.
    #[inline]
    pub fn expect_consensus(&self) -> &Consensus<S> {
        self.consensus().expect("no consensus!")
    }

    #[inline]
    #[doc(hidden)]
    pub fn has_consensus(&self) -> bool {
        self.consensus().is_some()
    }

    pub async fn start_services(&self) {
        let self_clone = self.clone();
        let mut receiver = self.inbound.take_receiver();
        let incoming_task = task::spawn(async move {
            loop {
                if let Err(e) = self_clone.process_incoming_messages(&mut receiver).await {
                    error!("Node error: {}", e);
                }
            }
        });
        self.register_task(incoming_task);

        let self_clone = self.clone();
        let peer_sync_interval = self.config.peer_sync_interval();
        let peering_task = task::spawn(async move {
            loop {
                info!("Updating peers");

                if let Err(e) = self_clone.update_peers().await {
                    error!("Peer update error: {}", e);
                }
                sleep(peer_sync_interval).await;
            }
        });
        self.register_task(peering_task);

        if !self.config.is_bootnode() {
            let self_clone = self.clone();
            let state_mgmt_task = task::spawn(async move {
                loop {
                    sleep(std::time::Duration::from_secs(5)).await;

                    // make sure that the node doesn't remain in a sync state without peers
                    if self_clone.state() == State::Syncing && self_clone.peer_book.number_of_connected_peers() == 0 {
                        self_clone.set_state(State::Idle);
                    }

                    // report node's current state
                    trace!("Node state: {:?}", self_clone.state());
                }
            });
            self.register_task(state_mgmt_task);

            if let Some(ref consensus) = self.consensus() {
                let self_clone = self.clone();
                let consensus = Arc::clone(consensus);
                let transaction_sync_interval = consensus.transaction_sync_interval();
                let tx_sync_task = task::spawn(async move {
                    loop {
                        sleep(transaction_sync_interval).await;

                        if !consensus.is_syncing_blocks() {
                            info!("Updating transactions");

                            // select last seen node as block sync node
                            let sync_node = self_clone.peer_book.last_seen();
                            consensus.update_transactions(sync_node).await;
                        }
                    }
                });
                self.register_task(tx_sync_task);
            }
        }
    }

    pub fn shut_down(&self) {
        debug!("Shutting down");

        for addr in self.connected_addrs() {
            let _ = self.disconnect_from_peer(addr);
        }

        for handle in self.threads.lock().drain(..).rev() {
            let _ = handle.join().map_err(|e| error!("Can't join a thread: {:?}", e));
        }

        for handle in self.tasks.lock().drain(..).rev() {
            handle.abort();
        }
    }

    pub fn register_task(&self, handle: task::JoinHandle<()>) {
        self.tasks.lock().push(handle);
    }

    pub fn register_thread(&self, handle: thread::JoinHandle<()>) {
        self.threads.lock().push(handle);
    }

    #[inline]
    pub fn local_address(&self) -> Option<SocketAddr> {
        self.local_address.get().copied()
    }

    #[inline]
    pub fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Relaxed)
    }

    /// Sets the local address of the node to the given value.
    #[inline]
    pub fn set_local_address(&self, addr: SocketAddr) {
        self.local_address
            .set(addr)
            .expect("local address was set more than once!");
    }
}
