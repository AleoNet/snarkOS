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

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, oneshot},
    time::sleep,
};

pub type ResourceId = usize;

/// The types of messages that the resource manager can handle.
pub enum ResourceRequest {
    /// A request to register the given resource under the given id.
    Register(Resource, ResourceId),
    /// A request to deregister the resource with the given id.
    Deregister(ResourceId),
    /// A request to deregister all the resources.
    Shutdown,
    /// A request to print a resource use summary in the logs.
    #[cfg(test)]
    LogSummary,
}

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
    sender: mpsc::UnboundedSender<ResourceRequest>,
    index: Arc<AtomicUsize>,
}

impl Clone for Resources {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            index: self.index.clone(),
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
            let mut resources: HashMap<ResourceId, Resource> = Default::default();
            while let Some(request) = receiver.recv().await {
                match request {
                    ResourceRequest::Register(resource, id) => {
                        if resources.insert(id, resource).is_some() {
                            error!("A resource with ID {} already exists!", id);
                        }
                    }
                    ResourceRequest::Deregister(id) => {
                        if let Some(resource) = resources.remove(&id) {
                            // Spawn a short-lived task that will allow some time for the
                            // to-be-aborted task to free its resources.
                            tokio::spawn(async move {
                                sleep(Duration::from_secs(1)).await;
                                resource.abort().await;
                            });
                        } else {
                            error!("Resource with ID {} was not found", id);
                        }
                    }
                    ResourceRequest::Shutdown => break,
                    #[cfg(test)]
                    ResourceRequest::LogSummary => {
                        let (mut num_tasks, mut num_threads) = (0, 0);
                        for resource in resources.values() {
                            match resource {
                                Resource::Task(..) => num_tasks += 1,
                                Resource::Thread(..) => num_threads += 1,
                            }
                        }
                        debug!("[Resources] dedicated threads: {}, dedicated tasks: {}", num_threads, num_tasks);
                    }
                }
            }

            let mut resources = resources.into_iter().collect::<Vec<_>>();
            resources.sort_unstable_by_key(|(id, _res)| *id);

            for (id, resource) in resources.into_iter().rev() {
                trace!("Aborting resource with ID {}", id);
                resource.abort().await;
            }
        });

        Self {
            sender,
            index: Default::default(),
        }
    }

    /// Obtains an ID that can be used to register a resource.
    pub fn procure_id(&self) -> ResourceId {
        self.index.fetch_add(1, Ordering::SeqCst)
    }

    /// Register the given resource with the resource handler and optionally
    /// with an associated resource id which - if provided - must be obtained
    /// earlier via `Resources::procure_id`.
    pub fn register(&self, resource: Resource, id: Option<ResourceId>) {
        // Prepare a resource request, assigning a resource ID if one was not given.
        let request = ResourceRequest::Register(resource, match id {
            Some(id) => id,
            None => self.procure_id(),
        });
        // This channel will not be closed before the final shutdown.
        let _ = self.sender.send(request);
    }

    /// Register the given task with the resource handler and an (optional) resource id.
    pub fn register_task(&self, id: Option<ResourceId>, handle: tokio::task::JoinHandle<()>) {
        self.register(Resource::Task(handle), id);
    }

    /// Register the given thread with the resource handler and an (optional) resource id.
    pub fn register_thread(&self, id: Option<ResourceId>, handle: std::thread::JoinHandle<()>, abort_sender: AbortSignal) {
        self.register(Resource::Thread(handle, abort_sender), id);
    }

    /// Register a task and instrument it with the given `name`; useful metrics on it will be logged every `interval`.
    #[cfg(feature = "task-metrics")]
    pub fn register_instrumented_task(
        &self,
        id: Option<ResourceId>,
        name: String,
        interval: std::time::Duration,
        handle: tokio::task::JoinHandle<()>,
    ) {
        let task_monitor = tokio_metrics::TaskMonitor::new();

        let tm = task_monitor.clone();
        self.register_task(
            None,
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    let metrics = tm.cumulative(); // Note: the list of metrics below is not exhaustive.
                    debug!(
                        "['{}' task metrics] mean poll duration: {:?}; mean idle duration: {:?}; slow poll ratio: {:?}",
                        name,
                        metrics.mean_poll_duration(),
                        metrics.mean_idle_duration(),
                        metrics.slow_poll_ratio(),
                    );
                }
            }),
        );

        self.register_task(
            id,
            tokio::spawn(async move {
                let _ = task_monitor.instrument(handle).await;
            }),
        );
    }

    /// Deregisters and frees a resource with the given ID.
    pub fn deregister(&self, id: ResourceId) {
        let request = ResourceRequest::Deregister(id);

        // This channel will not be closed before the final shutdown.
        let _ = self.sender.send(request);
    }

    /// Terminates all processes related to registered resources, freeing them in the process.
    pub fn shut_down(&self) {
        // This channel will not be closed before the final shutdown.
        let _ = self.sender.send(ResourceRequest::Shutdown);
    }

    #[cfg(test)]
    fn log_summary(&self) {
        // This channel will not be closed before the final shutdown.
        let _ = self.sender.send(ResourceRequest::LogSummary);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use tokio::sync::oneshot;

    struct DropChecker(usize);

    impl Drop for DropChecker {
        fn drop(&mut self) {
            warn!("Dropping {}", self.0);
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "This test is for inspection purposes only."]
    async fn resource_management() {
        tracing_subscriber::fmt::init();

        let resources = Resources::default();
        resources.log_summary();

        let task_id0 = resources.procure_id();
        let task0 = tokio::task::spawn(async move {
            let _drop_checker = DropChecker(task_id0);
            loop {
                sleep(Duration::from_millis(10)).await;
            }
        });
        resources.register_task(Some(task_id0), task0);

        resources.log_summary();

        let resources_clone = resources.clone();
        let task_id1 = resources.procure_id();
        let task1 = tokio::task::spawn(async move {
            let _drop_checker = DropChecker(task_id1);

            sleep(Duration::from_millis(1)).await;

            resources_clone.deregister(task_id1);
        });
        resources.register_task(Some(task_id1), task1);

        resources.log_summary();

        let thread_id = resources.procure_id();
        let (tx, mut rx) = oneshot::channel();
        let thread = thread::spawn(move || {
            let _drop_checker = DropChecker(thread_id);
            loop {
                if rx.try_recv().is_ok() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        resources.register_thread(Some(thread_id), thread, tx);

        thread::sleep(Duration::from_millis(10));

        resources.log_summary();
        resources.deregister(task_id0);
        resources.log_summary();
        resources.deregister(thread_id);
        resources.log_summary();

        sleep(Duration::from_millis(100)).await;

        resources.log_summary();
    }
}
