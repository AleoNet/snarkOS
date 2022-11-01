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

use std::collections::VecDeque;
use tokio::sync::mpsc;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub(crate) struct Logs {
    log_receiver: mpsc::Receiver<Vec<u8>>,
    log_cache: VecDeque<String>,
    log_limit: usize,
}

impl Logs {
    pub(crate) fn new(log_receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        let log_limit = 128; // an arbitrary number fitting the testing terminal room

        Self { log_receiver, log_cache: VecDeque::with_capacity(log_limit), log_limit }
    }

    pub(crate) fn draw<B: Backend>(&mut self, f: &mut Frame<B>, area: Rect) {
        // Initialize the layout of the page.
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(area);

        let mut new_logs = Vec::new();
        while let Ok(log) = self.log_receiver.try_recv() {
            new_logs.push(match String::from_utf8(log) {
                Ok(log) => log,
                _ => String::new(),
            });
        }

        let all_logs = self.log_cache.len() + new_logs.len();
        if all_logs > self.log_limit {
            let remaining_room = self.log_limit - self.log_cache.len();
            let overflow = all_logs - self.log_cache.len();

            if overflow > self.log_limit {
                self.log_cache.clear();
            } else {
                let missing_room = all_logs - remaining_room;
                for _ in 0..missing_room {
                    self.log_cache.pop_front();
                }
            }
        };

        self.log_cache.extend(new_logs.into_iter().take(self.log_limit));

        let combined_logs = self.log_cache.iter().map(|s| s.as_str()).collect::<String>();

        let combined_logs = Paragraph::new(combined_logs).block(Block::default().borders(Borders::ALL).title("Logs"));
        f.render_widget(combined_logs, chunks[0]);
    }
}
