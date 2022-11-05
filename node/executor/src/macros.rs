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

#[macro_export]
macro_rules! spawn_task {
    // Spawns a new task, with a task ID, using a custom executor.
    ($E:ident, $logic:block) => {{
        // Procure a resource ID for the task, as it may terminate at any time.
        let resource_id = $E::resources().procure_id();

        // Initialize a handler for the task.
        let (router, handler) = tokio::sync::oneshot::channel();

        // Register the task with the environment.
        $E::resources().register_task(
            Some(resource_id),
            tokio::task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());

                let result = $logic;

                // Unregister the task from the environment.
                $E::resources().deregister(resource_id);

                result
            }),
        );

        // Wait until the task is ready.
        let _ = handler.await;
    }};

    // Spawns a new task, with a task ID, using a custom executor.
    ($E:ident, $logic:expr) => {{ $crate::spawn_task!($E, { $logic }) }};
}

#[macro_export]
macro_rules! spawn_task_loop {
    // Spawns a new task, without a task ID, using a custom executor.
    ($E:ident, $logic:block) => {{
        // Initialize a handler for the task.
        let (router, handler) = tokio::sync::oneshot::channel();

        // Register the task with the environment.
        // No need to provide an id, as the task will run indefinitely.
        $E::resources().register_task(
            None,
            tokio::task::spawn(async move {
                // Notify the outer function that the task is ready.
                let _ = router.send(());
                $logic
            }),
        );

        // Wait until the task is ready.
        let _ = handler.await;
    }};

    // Spawns a new task, without a task ID, using a custom executor.
    ($E:ident, $logic:expr) => {{ $crate::spawn_task!($E, None, { $logic }) }};
}

#[macro_export]
macro_rules! spawn_task_away {
    ($logic:block) => {{ tokio::task::spawn(async move { $logic }) }};

    // Spawns a new task, with a task ID, using a custom executor.
    ($logic:expr) => {{ $crate::spawn_task!({ $logic }) }};
}
