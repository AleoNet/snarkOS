// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use tokio::sync::{mpsc, oneshot};

/// Provides the means to shut down a background resource.
pub type AbortSignal = oneshot::Sender<()>;

/// A background task or thread.
pub enum Resource {
    /// An asynchronous task.
    Task(tokio::task::JoinHandle<()>),
    /// An OS-native thread.
    Thread(std::thread::JoinHandle<()>, AbortSignal),
}

impl Resource {
    /// Terminate a process related to a resource, freeing it in the process.
    pub async fn abort(self) {
        match self {
            Self::Task(handle) => {
                handle.abort();
            }
            Self::Thread(handle, sender) => {
                let _ = sender.send(());
                let _ = tokio::task::spawn_blocking(move || {
                    let _ = handle.join().map_err(|e| error!("Can't join a thread: {:?}", e));
                })
                .await;
            }
        }
    }
}

/// A collection of handles to resources bound to active processes.
#[derive(Debug)]
pub struct Resources {
    sender: mpsc::UnboundedSender<Option<Resource>>,
}

impl Clone for Resources {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl Default for Resources {
    fn default() -> Self {
        Self::new()
    }
}

impl Resources {
    /// Create an instance of the resource handler.
    pub fn new() -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        tokio::spawn(async move {
            let mut resources: Vec<Resource> = vec![];
            while let Some(resource) = receiver.recv().await {
                match resource {
                    Some(resource) => resources.push(resource),
                    None => break,
                }
            }
            for resource in resources {
                resource.abort().await;
            }
        });

        Self { sender }
    }

    /// Register the given resource with the resource handler.
    pub fn register(&self, resource: Resource) {
        let _ = self.sender.send(Some(resource));
    }

    /// Register the given task with the resource handler.
    pub fn register_task(&self, handle: tokio::task::JoinHandle<()>) {
        let task = Resource::Task(handle);
        self.register(task);
    }

    /// Register the given thread with the resource handler.
    pub fn register_thread(&self, handle: std::thread::JoinHandle<()>, abort_sender: AbortSignal) {
        let thread = Resource::Thread(handle, abort_sender);
        self.register(thread);
    }

    /// Terminate all processes related to registered resources, freeing them in the process.
    pub fn abort(&self) {
        self.sender.send(None).ok();
    }
}
