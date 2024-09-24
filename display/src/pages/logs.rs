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

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::VecDeque;
use tokio::sync::mpsc;

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

    pub(crate) fn draw(&mut self, f: &mut Frame, area: Rect) {
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
