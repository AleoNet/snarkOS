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

use tokio::sync::mpsc;

pub trait DropJoinable: Send + Sync + 'static {
    fn destroy(self);
}

impl<X: Send + Sync + 'static> DropJoinable for tokio::task::JoinHandle<X> {
    fn destroy(self) {
        self.abort();
    }
}

impl<X: Send + Sync + 'static> DropJoinable for std::thread::JoinHandle<X> {
    fn destroy(self) {
        tokio::task::spawn_blocking(move || {
            self.join().map_err(|e| error!("Can't join a thread: {:?}", e)).ok();
        });
    }
}

#[derive(Debug)]
pub struct DropJoin<T: DropJoinable> {
    sender: mpsc::UnboundedSender<Option<T>>,
}

impl<T: DropJoinable> Clone for DropJoin<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T: DropJoinable> Default for DropJoin<T> {
    fn default() -> Self {
        DropJoin::new()
    }
}

impl<T: DropJoinable> DropJoin<T> {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        tokio::spawn(Self::drop_listener(receiver));
        DropJoin { sender }
    }

    pub fn append(&self, item: T) {
        self.sender.send(Some(item)).ok();
    }

    pub fn flush(&self) {
        self.sender.send(None).ok();
    }

    async fn drop_listener(mut receiver: mpsc::UnboundedReceiver<Option<T>>) {
        let mut values = vec![];
        while let Some(value) = receiver.recv().await {
            match value {
                Some(x) => values.push(x),
                None => {
                    for value in values.drain(..) {
                        value.destroy();
                    }
                }
            }
        }
        for value in values {
            value.destroy();
        }
    }
}
