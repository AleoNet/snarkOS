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
    /// The local address of this node.
    pub local_address: OnceCell<SocketAddr>,
    /// The pre-configured parameters of this node.
    pub config: Config,
    /// The inbound handler of this node.
    pub inbound: Inbound,
    /// The outbound handler of this node.
    pub outbound: Outbound,
    /// The list of connected and disconnected peers of this node.
    pub peer_book: PeerBook,
    /// The sync handler of this node.
    pub sync: OnceCell<Arc<Sync<S>>>,
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

impl<S: Storage + Send + core::marker::Sync + 'static> Node<S> {
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
            sync: Default::default(),
            tasks: Default::default(),
            threads: Default::default(),
            shutting_down: Default::default(),
        })))
    }

    pub fn set_sync(&mut self, sync: Sync<S>) {
        if self.sync.set(Arc::new(sync)).is_err() {
            panic!("sync was set more than once!");
        }
    }

    /// Returns a reference to the sync objects.
    #[inline]
    pub fn sync(&self) -> Option<&Arc<Sync<S>>> {
        self.sync.get()
    }

    /// Returns a reference to the sync objects, expecting them to be available.
    #[inline]
    pub fn expect_sync(&self) -> &Sync<S> {
        self.sync().expect("no sync!")
    }

    #[inline]
    #[doc(hidden)]
    pub fn has_sync(&self) -> bool {
        self.sync().is_some()
    }

    pub async fn start_services(&self) {
        let node_clone = self.clone();
        let mut receiver = self.inbound.take_receiver();
        let incoming_task = task::spawn(async move {
            loop {
                if let Err(e) = node_clone.process_incoming_messages(&mut receiver).await {
                    error!("Node error: {}", e);
                }
            }
        });
        self.register_task(incoming_task);

        let node_clone = self.clone();
        let peer_sync_interval = self.config.peer_sync_interval();
        let peering_task = task::spawn(async move {
            loop {
                info!("Updating peers");

                if let Err(e) = node_clone.update_peers().await {
                    error!("Peer update error: {}", e);
                }
                sleep(peer_sync_interval).await;
            }
        });
        self.register_task(peering_task);

        if !self.config.is_bootnode() {
            let node_clone = self.clone();
            let state_tracking_task = task::spawn(async move {
                loop {
                    sleep(std::time::Duration::from_secs(5)).await;

                    // Make sure that the node doesn't remain in a sync state without peers.
                    if node_clone.state() == State::Syncing && node_clone.peer_book.number_of_connected_peers() == 0 {
                        node_clone.set_state(State::Idle);
                    }

                    // Report node's current state.
                    trace!("Node state: {:?}", node_clone.state());
                }
            });
            self.register_task(state_tracking_task);

            if let Some(ref sync) = self.sync() {
                let node_clone = self.clone();
                let bootnodes = self.config.bootnodes();
                let sync = Arc::clone(sync);
                let mempool_sync_interval = sync.mempool_sync_interval();
                let sync_task = task::spawn(async move {
                    loop {
                        sleep(mempool_sync_interval).await;

                        if !sync.is_syncing_blocks() {
                            // The order of preference for the sync node is as follows:
                            //   1. Iterate (in declared order) through the bootnodes:
                            //      a. Check if this node is connected to the specified bootnode in the peer book.
                            //      b. Select the specified bootnode as the sync node if this node is connected to it.
                            //   2. If this node is not connected to any bootnode,
                            //      then select the last seen peer as the sync node.

                            // Step 1.
                            let mut sync_node = None;
                            for bootnode in bootnodes.iter() {
                                if node_clone.peer_book.is_connected(*bootnode) {
                                    sync_node = Some(*bootnode);
                                    break;
                                }
                            }

                            // Step 2.
                            if sync_node.is_none() {
                                // Select last seen node as block sync node.
                                sync_node = node_clone.peer_book.last_seen();
                            }

                            sync.update_memory_pool(sync_node).await;
                        }
                    }
                });
                self.register_task(sync_task);
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
